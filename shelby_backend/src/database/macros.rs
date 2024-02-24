macro_rules! question_mark {
    ($name: ident) => {
        "?"
    };
}

/// Create a indexable database entry.
macro_rules! make_struct {
    ($(
    #[derive($( $derived: ty ),+)] )?
    #[table($table_name: expr)]
    #[dependencies( $dependencies: ty )]
    $name: ident { $( $(#[$os_attr: meta])? $element: ident: $ty: ty),* } $( ($additional_conditions: expr) )?) => {
        paste::paste! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            $(#[derive($( $derived ),+)])?
            pub struct $name {
                $( $(#[$os_attr])? pub $element: $ty),*
            }

            impl crate::database::DatabaseEntry for $name {
                type DependsOn = $dependencies;

                const TABLE_NAME: &'static str = $table_name;
                const STATEMENT_CREATE_TABLE: &'static str = const_format::concatcp!(
                    "CREATE TABLE IF NOT EXISTS ", $table_name, " (id INTEGER PRIMARY KEY", $( ", ", stringify!($element), " ", <$ty as crate::database::DatabaseType>::COLUMN_VALUE ),* $(, ", ", $additional_conditions, " ")? , " )"
                );
            }

            impl crate::database::Insertable for $name {
                const STATEMENT_INSERT: &'static str = std::concat!(
                    "INSERT INTO ", $table_name, " (", concat_with::concat!(with ", ", $(stringify!($element)),*), ") VALUES (", concat_with::concat!(with ", ", $(crate::database::question_mark!($element)),*), ")"
                );

                type InsertValue<'a> = ($( &'a $ty ),*, );

                fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a> {
                    ($( &self.$element ),* ,)
                }
            }

            impl crate::database::Indexable for $name {}

            impl crate::database::Selectable for $name {
                type Output = crate::database::Record<Self>;

                const STATEMENT_SELECT_ALL: &'static str = std::concat!("SELECT id, ", concat_with::concat!(with ", ", $(stringify!($element)),*) ," FROM ", $table_name);

                type SelectValue<'a> = (i64, $( $ty ),*);

                fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> crate::database::Record<Self> {
                    let (primary_key, $( $element ),*) = value;
                    crate::database::Record {
                        identifier: crate::database::PrimaryKey::from(primary_key),
                        value: $name { $( $element ),* }
                    }
                }
            }

            impl crate::database::SelectableByPrimaryKey for $name {
                const STATEMENT_SELECT: &'static str = std::concat!("SELECT id, ", concat_with::concat!(with ", ", $(stringify!($element)),*) ," FROM ", $table_name, " WHERE id = ?");

                /// Select an element and parse it.
                fn select(database: &crate::database::Database, index: PrimaryKey<Self>) -> Result<Self::Output, crate::database::Error> {
                    Ok(database
                        .connection
                        .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                            Self::SelectValue::try_from(row).map(<Self as crate::database::Selectable>::deserialize_sql)
                        })?)
                }

                /// Try to select a element which primary key was not validated.
                fn try_select(database: &crate::database::Database, index: i64) -> Result<Option<Self::Output>, crate::database::Error> {
                    use rusqlite::OptionalExtension;

                    Ok(database
                        .connection
                        .query_row(Self::STATEMENT_SELECT, (index,), |row| {
                            Self::SelectValue::try_from(row).map(<Self as crate::database::Selectable>::deserialize_sql)
                        })
                        .optional()?)
                }
            }

            #[cfg(test)]
            mod [< "test_" $table_name >] {
                use crate::database::{DatabaseEntry, Selectable, Insertable, DefaultGenerator, SelectableByPrimaryKey};
                use super::$name;

                #[test]
                fn test_insert_automatically() {
                    let database = crate::database::Database::plain().expect("valid database");
                    $name::create_table(&database).expect("valid table");
                    $name::create_default(&database).insert(&database).expect("insert sucessfull");
                }

                #[test]
                fn test_select_automatically() {
                    let database = crate::database::Database::plain().expect("valid database");
                    $name::create_table(&database).expect("valid table");

                    let example = $name::create_default(&database);
                    let id = example.insert(&database).expect("insert sucessfull");
                    let loaded_example = $name::select(&database, id).expect("valid sample");

                    assert_eq!(example, loaded_example.value)
                }

                #[test]
                fn test_select_all() {
                    let database = crate::database::Database::plain().expect("valid database");
                    $name::create_table(&database).expect("valid table");

                    let example = $name::create_default(&database);
                    example.insert(&database).expect("insert sucessfull");

                    let loaded_examples = $name::select_all(&database).expect("valid sample");
                    assert_eq!(loaded_examples.len(), 1);
                    assert_eq!(example, loaded_examples[0].value)
                }

                #[test]
                fn test_select_raw() {
                    let database = crate::database::Database::plain().expect("valid database");
                    $name::create_table(&database).expect("valid table");

                    let example = $name::create_default(&database);
                    let id = example.insert(&database).expect("insert sucessfull");

                    let loaded_example = $name::try_select(&database, id.0).expect("valid sample");
                    assert_eq!(example, loaded_example.expect("existing element").value)
                }

                #[test]
                fn test_select_raw_noexisting() {
                    const NONEXISTING_INDEX: i64 = 42;

                    let database = crate::database::Database::plain().expect("valid database");
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
    use crate::database::{
        Database, DatabaseEntry, Insertable, PrimaryKey, Record, SelectableByPrimaryKey,
    };

    crate::database::make_struct!(
        #[derive(Default, serde::Serialize, serde::Deserialize)]
        #[table("tests")]
        #[dependencies(())]
        Test {
            bool_value: bool,
            string_value: String,
            integer_value: u32
        }
    );

    // We need to check values with single elements are properly serialized, too.
    crate::database::make_struct!(
        #[derive(Default)]
        #[table("tests_single")]
        #[dependencies(())]
        TestSingleElement {
            string_value: String
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
            "CREATE TABLE IF NOT EXISTS tests (id INTEGER PRIMARY KEY, bool_value BOOL NOT NULL, string_value TEXT NOT NULL, integer_value INTEGER NOT NULL )"
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
            identifier: crate::database::PrimaryKey::from(0),
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
            identifier: crate::database::PrimaryKey::from(0),
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
        let primary_key: PrimaryKey<Test> = crate::database::PrimaryKey::from(0);
        assert_eq!(
            serde_json::to_string(&primary_key).expect("str"),
            "\"/tests/0\""
        )
    }

    #[test]
    fn primary_key_deserialization() {
        const TEST_INPUT: &'static str = "\"/tests/42\"";
        assert_eq!(
            serde_json::from_str::<crate::database::PrimaryKey<Test>>(TEST_INPUT)
                .expect("valid key"),
            crate::database::PrimaryKey::from(42)
        );
    }
}
