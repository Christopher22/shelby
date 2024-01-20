mod database;
pub mod person;
pub mod user;

use serde::{Deserialize, Serialize};

pub use self::database::Database;

/// A record with associated, numerical primary key.
#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T>
where
    T: IndexableDatebaseEntry,
{
    pub identifier: PrimaryKey<T>,
    pub value: T,
}

impl<T> std::ops::Deref for Record<T>
where
    T: IndexableDatebaseEntry,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// The primary key of a record.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimaryKey<T: IndexableDatebaseEntry>(pub(crate) i64, std::marker::PhantomData<T>);

impl<T: IndexableDatebaseEntry> From<i64> for PrimaryKey<T> {
    fn from(value: i64) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

impl<T: IndexableDatebaseEntry> rusqlite::ToSql for PrimaryKey<T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(self.0.into()))
    }
}

impl<T: IndexableDatebaseEntry> rusqlite::types::FromSql for PrimaryKey<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).map(PrimaryKey::from)
    }
}

#[cfg(test)]
impl<T: IndexableDatebaseEntry> Default for PrimaryKey<T> {
    fn default() -> Self {
        Self(0, std::marker::PhantomData)
    }
}

pub trait Dependency {
    fn create_dependencies(database: &Database) -> Result<(), rusqlite::Error>;
}

impl Dependency for () {
    fn create_dependencies(_: &Database) -> Result<(), rusqlite::Error> {
        Ok(())
    }
}

impl<T> Dependency for T
where
    T: DatabaseEntry,
{
    fn create_dependencies(database: &Database) -> Result<(), rusqlite::Error> {
        T::DependsOn::create_dependencies(database)?;
        database
            .connection
            .execute(T::STATEMENT_CREATE_TABLE, ())
            .map(|_| ())
    }
}

impl<T1, T2> Dependency for (T1, T2)
where
    T1: Dependency,
    T2: Dependency,
{
    fn create_dependencies(database: &Database) -> Result<(), rusqlite::Error> {
        T1::create_dependencies(database)?;
        T2::create_dependencies(database)
    }
}

/// An element serialialized in the Database.
pub trait DatabaseEntry: Sized {
    type DependsOn: Dependency;

    const TABLE_NAME: &'static str;
    const STATEMENT_CREATE_TABLE: &'static str;

    /// Create the required table and all dependencies.
    fn create_table(database: &Database) -> Result<(), rusqlite::Error> {
        Self::create_dependencies(database)
    }
}

/// An value insertable in the database.
pub trait IndexableDatebaseEntry: DatabaseEntry {
    /// The statement for select WITH explicit primary key.
    const STATEMENT_SELECT: &'static str;

    /// The statement for insert the InsertValue WITHOUT explicit primary key.
    const STATEMENT_INSERT: &'static str;

    /// A (mostly tuple-based) values which could be inserted.
    type InsertValue<'a>: rusqlite::Params
    where
        Self: 'a;

    /// The value which should be extracted from the row.
    type SelectValue<'a>: TryFrom<&'a rusqlite::Row<'a>, Error = rusqlite::Error>;

    /// Convert the value as the insert values.
    fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a>;

    /// Deserialize the database value into a Record.
    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> Record<Self>;

    /// Select an element and parse it.
    fn select(
        database: &crate::Database,
        index: PrimaryKey<Self>,
    ) -> Result<Record<Self>, rusqlite::Error> {
        database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })
    }

    /// Insert the value with a given primary key.
    fn insert(&self, database: &crate::Database) -> Result<PrimaryKey<Self>, rusqlite::Error> {
        database
            .connection
            .execute(Self::STATEMENT_INSERT, self.serialize_sql())
            .map(|_| PrimaryKey::from(database.connection.last_insert_rowid()))
    }
}

pub(crate) mod macros {
    macro_rules! question_mark {
        ($name: ident) => {
            "?"
        };
    }

    macro_rules! make_struct {
        ($name: ident (Table: $table_name: expr) depends on $dependencies: ty => { $($element: ident: $ty: ty => $value: expr),* } $( ($additional_conditions: expr) )?) => {
            paste::paste! {
                #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
                #[cfg_attr(test, derive(Default))]
                pub struct $name {
                    $( pub $element: $ty),*
                }

                impl crate::DatabaseEntry for $name {
                    type DependsOn = $dependencies;

                    const TABLE_NAME: &'static str = $table_name;
                    const STATEMENT_CREATE_TABLE: &'static str = std::concat!("CREATE TABLE IF NOT EXISTS ", $table_name, " (id INTEGER PRIMARY KEY", $( ", ", stringify!($element), " ", $value),* $(, ", ", $additional_conditions, " ")? , " )");
                }

                impl crate::IndexableDatebaseEntry for $name {
                    const STATEMENT_INSERT: &'static str = std::concat!("INSERT INTO ", $table_name, " (", concat_with::concat!(with ", ", $(stringify!($element)),*), ") VALUES (", concat_with::concat!(with ", ", $(crate::macros::question_mark!($element)),*), ")");
                    const STATEMENT_SELECT: &'static str = std::concat!("SELECT id, ", concat_with::concat!(with ", ", $(stringify!($element)),*) ," FROM ", $table_name, " WHERE id = ?");

                    type InsertValue<'a> = ($( &'a $ty ),*, );
                    type SelectValue<'a> = (i64, $( $ty ),*);

                    fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a> {
                        ($( &self.$element ),* ,)
                    }

                    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> crate::Record<Self> {
                        let (primary_key, $( $element ),*) = value;
                        crate::Record {
                            identifier: crate::PrimaryKey::from(primary_key),
                            value: $name { $( $element ),* }
                        }
                    }
                }

                #[cfg(test)]
                mod [< "test_" $table_name >] {
                    use crate::{DatabaseEntry, IndexableDatebaseEntry};
                    use super::$name;

                    #[test]
                    fn test_insert_automatically() {
                        let database = crate::Database::from_memory().expect("valid database");
                        $name::create_table(&database).expect("valid table");
                        $name::default().insert(&database).expect("insert sucessfull");
                    }

                    #[test]
                    fn test_select_automatically() {
                        let database = crate::Database::from_memory().expect("valid database");
                        $name::create_table(&database).expect("valid table");

                        let example = $name::default();
                        let id = example.insert(&database).expect("insert sucessfull");
                        let loaded_example = $name::select(&database, id).expect("valid sample");

                        assert_eq!(example, loaded_example.value)
                    }
                }
            }
        }
    }

    pub(crate) use make_struct;
    pub(crate) use question_mark;

    #[cfg(test)]
    mod test {
        use crate::{Database, DatabaseEntry, IndexableDatebaseEntry};

        crate::macros::make_struct!(
            Test (Table: "tests") depends on () => {
                bool_value: bool => "BOOL NOT NULL",
                string_value: String  => "STRING NOT NULL",
                integer_value: u32 => "INTEGER NOT NULL"
            }
        );

        // We need to check values with single elements are properly serialized, too.
        crate::macros::make_struct!(
            TestSingleElement (Table: "tests_single") depends on () => {
                string_value: String  => "STRING NOT NULL"
            }
        );

        #[test]
        fn test_table_name() {
            assert_eq!(Test::TABLE_NAME, "tests");
        }

        #[test]
        fn test_create_table_statement() {
            assert_eq!(
                Test::STATEMENT_CREATE_TABLE,
                "CREATE TABLE IF NOT EXISTS tests (id INTEGER PRIMARY KEY, bool_value BOOL NOT NULL, string_value STRING NOT NULL, integer_value INTEGER NOT NULL )"
            );
        }

        #[test]
        fn test_insert_statement() {
            assert_eq!(
                Test::STATEMENT_INSERT,
                "INSERT INTO tests (bool_value, string_value, integer_value) VALUES (?, ?, ?)"
            );
        }

        #[test]
        fn test_select_statement() {
            assert_eq!(
                Test::STATEMENT_SELECT,
                "SELECT id, bool_value, string_value, integer_value FROM tests WHERE id = ?"
            );
        }

        #[test]
        fn test_insert() {
            let database = Database::from_memory().expect("valid database");
            Test::create_table(&database).expect("valid table");

            Test {
                bool_value: false,
                string_value: String::from("ABC"),
                integer_value: 42,
            }
            .insert(&database)
            .expect("insert sucessfull");
        }
    }
}
