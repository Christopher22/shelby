mod database;
pub mod document;
mod error;
pub mod person;
pub mod user;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

pub use self::database::Database;
pub use self::error::Error;

/// A record with associated, numerical primary key.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<T>
where
    T: IndexableDatebaseEntry,
{
    pub identifier: PrimaryKey<T>,
    #[serde(flatten)]
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
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryKey<T: IndexableDatebaseEntry>(
    pub(crate) i64,
    std::marker::PhantomData<*const T>,
);

unsafe impl<T: IndexableDatebaseEntry> std::marker::Send for PrimaryKey<T> {}
unsafe impl<T: IndexableDatebaseEntry> std::marker::Sync for PrimaryKey<T> {}

#[derive(Default)]
struct PrimaryKeyVisitor<T: IndexableDatebaseEntry>(std::marker::PhantomData<*const T>);

impl<'de, T: IndexableDatebaseEntry> serde::de::Visitor<'de> for PrimaryKeyVisitor<T> {
    type Value = PrimaryKey<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a path in the form of '/{}/(<number>' or a u64",
            T::TABLE_NAME
        )
    }

    fn visit_i64<E>(self, primary_key: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PrimaryKey::from(primary_key))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut iterator = v.split('/').rev();

        let primary_key: i64 =
            iterator
                .next()
                .and_then(|value| value.parse().ok())
                .ok_or(E::invalid_value(
                    serde::de::Unexpected::Other("invalid number"),
                    &self,
                ))?;

        match iterator.next() {
            Some(value) if value == T::TABLE_NAME => Ok(PrimaryKey::from(primary_key)),
            Some(_) => Err(E::invalid_value(
                serde::de::Unexpected::Other("invalid identifier"),
                &self,
            )),
            None => Err(E::invalid_value(
                serde::de::Unexpected::Other("invalid path"),
                &self,
            )),
        }
    }
}

impl<'de, T: IndexableDatebaseEntry> serde::Deserialize<'de> for PrimaryKey<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(PrimaryKeyVisitor::<T>(std::marker::PhantomData))
    }
}

impl<T: IndexableDatebaseEntry> serde::Serialize for PrimaryKey<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<T: IndexableDatebaseEntry> std::fmt::Display for PrimaryKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}/{}", T::TABLE_NAME, self.0)
    }
}

impl<T: IndexableDatebaseEntry> Clone for PrimaryKey<T> {
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
    }
}

impl<T: IndexableDatebaseEntry> Copy for PrimaryKey<T> {}

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

pub trait Dependency {
    fn create_dependencies(database: &Database) -> Result<(), Error>;
}

impl Dependency for () {
    fn create_dependencies(_: &Database) -> Result<(), Error> {
        Ok(())
    }
}

impl<T> Dependency for T
where
    T: DatabaseEntry,
{
    fn create_dependencies(database: &Database) -> Result<(), Error> {
        T::DependsOn::create_dependencies(database)?;
        Ok(database
            .connection
            .execute(T::STATEMENT_CREATE_TABLE, ())
            .map(|_| ())?)
    }
}

impl<T1, T2> Dependency for (T1, T2)
where
    T1: Dependency,
    T2: Dependency,
{
    fn create_dependencies(database: &Database) -> Result<(), Error> {
        T1::create_dependencies(database)?;
        Ok(T2::create_dependencies(database)?)
    }
}

/// An element serialialized in the Database.
pub trait DatabaseEntry: Sized {
    type DependsOn: Dependency;

    const TABLE_NAME: &'static str;
    const STATEMENT_CREATE_TABLE: &'static str;

    /// Create the required table and all dependencies.
    fn create_table(database: &Database) -> Result<(), Error> {
        Ok(Self::create_dependencies(database)?)
    }
}

/// An value insertable in the database.
pub trait IndexableDatebaseEntry: DatabaseEntry {
    /// The statement for select WITH explicit primary key.
    const STATEMENT_SELECT: &'static str;

    /// The statement for selecting all entries.
    const STATEMENT_SELECT_ALL: &'static str;

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
    fn select(database: &crate::Database, index: PrimaryKey<Self>) -> Result<Record<Self>, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })?)
    }

    /// Try to select a element which primary key was not validated.
    fn try_select(database: &crate::Database, index: i64) -> Result<Option<Record<Self>>, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })
            .optional()?)
    }

    /// Select all the elements from the database.
    fn select_all(database: &crate::Database) -> Result<Vec<Record<Self>>, Error> {
        let mut stmt = database.connection.prepare(Self::STATEMENT_SELECT_ALL)?;
        let iterator = stmt.query_map((), |row| {
            Self::SelectValue::try_from(row).map(Self::deserialize_sql)
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }

    /// Insert the value with a given primary key.
    fn insert(&self, database: &crate::Database) -> Result<PrimaryKey<Self>, Error> {
        Ok(database
            .connection
            .execute(Self::STATEMENT_INSERT, self.serialize_sql())
            .map(|_| PrimaryKey::from(database.connection.last_insert_rowid()))?)
    }
}

/// An trait for creating a default for objects with complex constraints like foreign keys requiering database access.
pub trait DefaultGenerator: DatabaseEntry {
    /// Create the default element.
    fn create_default(database: &crate::Database) -> Self;
}

impl<T: DatabaseEntry + Default> DefaultGenerator for T {
    fn create_default(_: &crate::Database) -> Self {
        Default::default()
    }
}

pub(crate) mod macros {
    macro_rules! question_mark {
        ($name: ident) => {
            "?"
        };
    }

    /// Create a indexable database entry.
    macro_rules! make_struct {
        ($name: ident (Table $( with derived $derived: ident )?: $table_name: expr) depends on $dependencies: ty => { $($element: ident: $ty: ty => $value: expr),* } $( ($additional_conditions: expr) )?) => {
            paste::paste! {
                #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
                $(#[derive($derived)])?
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
                    const STATEMENT_SELECT_ALL: &'static str = std::concat!("SELECT id, ", concat_with::concat!(with ", ", $(stringify!($element)),*) ," FROM ", $table_name);
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
                    use crate::{DatabaseEntry, IndexableDatebaseEntry, DefaultGenerator};
                    use super::$name;

                    #[test]
                    fn test_insert_automatically() {
                        let database = crate::Database::plain().expect("valid database");
                        $name::create_table(&database).expect("valid table");
                        $name::create_default(&database).insert(&database).expect("insert sucessfull");
                    }

                    #[test]
                    fn test_select_automatically() {
                        let database = crate::Database::plain().expect("valid database");
                        $name::create_table(&database).expect("valid table");

                        let example = $name::create_default(&database);
                        let id = example.insert(&database).expect("insert sucessfull");
                        let loaded_example = $name::select(&database, id).expect("valid sample");

                        assert_eq!(example, loaded_example.value)
                    }

                    #[test]
                    fn test_select_all() {
                        let database = crate::Database::plain().expect("valid database");
                        $name::create_table(&database).expect("valid table");

                        let example = $name::create_default(&database);
                        example.insert(&database).expect("insert sucessfull");

                        let loaded_examples = $name::select_all(&database).expect("valid sample");
                        assert_eq!(loaded_examples.len(), 1);
                        assert_eq!(example, loaded_examples[0].value)
                    }

                    #[test]
                    fn test_select_raw() {
                        let database = crate::Database::plain().expect("valid database");
                        $name::create_table(&database).expect("valid table");

                        let example = $name::create_default(&database);
                        let id = example.insert(&database).expect("insert sucessfull");

                        let loaded_example = $name::try_select(&database, id.0).expect("valid sample");
                        assert_eq!(example, loaded_example.expect("existing element").value)
                    }

                    #[test]
                    fn test_select_raw_noexisting() {
                        const NONEXISTING_INDEX: i64 = 42;

                        let database = crate::Database::plain().expect("valid database");
                        $name::create_table(&database).expect("valid table");

                        let index = $name::create_default(&database).insert(&database).expect("insert sucessfull");
                        assert_ne!(index.0, NONEXISTING_INDEX);

                        assert!($name::try_select(&database, NONEXISTING_INDEX).expect("valid sample").is_none());
                    }
                }
            }
        }
    }

    pub(crate) use make_struct;
    pub(crate) use question_mark;

    #[cfg(test)]
    mod test {
        use crate::{Database, DatabaseEntry, IndexableDatebaseEntry, PrimaryKey, Record};

        crate::macros::make_struct!(
            Test (Table with derived Default: "tests") depends on () => {
                bool_value: bool => "BOOL NOT NULL",
                string_value: String  => "STRING NOT NULL",
                integer_value: u32 => "INTEGER NOT NULL"
            }
        );

        // We need to check values with single elements are properly serialized, too.
        crate::macros::make_struct!(
            TestSingleElement (Table with derived Default: "tests_single") depends on () => {
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
            let database = Database::plain().expect("valid database");
            Test::create_table(&database).expect("valid table");

            Test {
                bool_value: false,
                string_value: String::from("ABC"),
                integer_value: 42,
            }
            .insert(&database)
            .expect("insert sucessfull");
        }

        #[test]
        fn test_serialization() {
            let record = Record {
                identifier: crate::PrimaryKey::from(0),
                value: Test {
                    bool_value: false,
                    string_value: String::from("ABC"),
                    integer_value: 42,
                },
            };

            assert_eq!(
                serde_json::to_string(&record).expect("valid serialization"),
                "{\"identifier\":\"/tests/0\",\"bool_value\":false,\"string_value\":\"ABC\",\"integer_value\":42}"
            );
        }

        #[test]
        fn test_deserialization() {
            let record = Record {
                identifier: crate::PrimaryKey::from(0),
                value: Test {
                    bool_value: false,
                    string_value: String::from("ABC"),
                    integer_value: 42,
                },
            };

            let serialized = serde_json::to_string(&record).expect("valid serialization");

            let deserialized: Record<Test> = serde_json::from_str(&serialized).expect("valid json");
            assert_eq!(record, deserialized);
        }

        #[test]
        fn primary_key_serialization() {
            let primary_key: PrimaryKey<Test> = crate::PrimaryKey::from(0);
            assert_eq!(
                serde_json::to_string(&primary_key).expect("str"),
                "\"/tests/0\""
            )
        }

        #[test]
        fn primary_key_deserialization() {
            const TEST_INPUT: &'static str = "\"/tests/42\"";
            assert_eq!(
                serde_json::from_str::<crate::PrimaryKey<Test>>(TEST_INPUT).expect("valid key"),
                crate::PrimaryKey::from(42)
            );
        }
    }
}
