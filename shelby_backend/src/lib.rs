mod database;

use serde::{Deserialize, Serialize};

pub use self::database::Database;

/// A record with associated, numerical primary key.
#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T>
where
    T: DatebaseEntry,
{
    pub identifier: PrimaryKey<T>,
    pub value: T,
}

/// The primary key of a record.
#[derive(Debug, Serialize, Deserialize)]
pub struct PrimaryKey<T: DatebaseEntry>(pub(crate) i64, std::marker::PhantomData<T>);

impl<T: DatebaseEntry> From<i64> for PrimaryKey<T> {
    fn from(value: i64) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

/// An element serialialized in the Database.
pub trait DatebaseEntry: Sized {
    const TABLE_NAME: &'static str;
    const STATEMENT_CREATE_TABLE: &'static str;
}

/// An value insertable in the database.
pub trait IndexableDatebaseEntry: DatebaseEntry {
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

#[allow(unused)]
pub(crate) mod macros {
    macro_rules! question_mark {
        ($name: ident) => {
            "?"
        };
    }

    macro_rules! make_struct {
        ($name: ident ($table_name: expr) => { $($element: ident: $ty: ty => $value: expr),* } ) => {
            paste::paste! {
                #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
                pub struct $name {
                    $( pub $element: $ty),*
                }

                impl crate::DatebaseEntry for $name {
                    const TABLE_NAME: &'static str = $table_name;
                    const STATEMENT_CREATE_TABLE: &'static str = std::concat!("CREATE TABLE ", $table_name, " (id INTEGER PRIMARY KEY", $( ", ", stringify!($element), " ", $value),*, " )");
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
                    use crate::IndexableDatebaseEntry;
                    use super::$name;

                    #[test]
                    fn test_insert_automatically() {
                        let database = crate::Database::from_memory().expect("valid database");
                        database.create_table::<$name>().expect("valid table");
                        $name::default().insert(&database).expect("insert sucessfull");
                    }

                    #[test]
                    fn test_select_automatically() {
                        let database = crate::Database::from_memory().expect("valid database");
                        database.create_table::<$name>().expect("valid table");

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
        use crate::{Database, DatebaseEntry, IndexableDatebaseEntry};

        crate::macros::make_struct!(
            Test ("tests") => {
                bool_value: bool => "BOOL NOT NULL",
                string_value: String  => "STRING NOT NULL",
                integer_value: u32 => "INTEGER NOT NULL"
            }
        );

        /// We need to check values with single elements are properly serialized, too.
        crate::macros::make_struct!(
            TestSingleElement ("tests_single") => {
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
                "CREATE TABLE tests (id INTEGER PRIMARY KEY, bool_value BOOL NOT NULL, string_value STRING NOT NULL, integer_value INTEGER NOT NULL )"
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
            database.create_table::<Test>().expect("valid table");
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
