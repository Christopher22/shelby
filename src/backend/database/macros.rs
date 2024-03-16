macro_rules! question_mark {
    ($name: ident) => {
        "?"
    };
}

macro_rules! impl_select {
    (false, $name: ident, $table_name: expr, $($element: ident: $ty: ty),*) => {};
    (true, $name: ident, $table_name: expr, $($element: ident: $ty: ty),*) => {
        impl crate::backend::database::Selectable for $name {
            type Output = crate::backend::database::Record<Self>;

            const STATEMENT_SELECT_ALL: &'static str = std::concat!("SELECT id, ", concat_with::concat!(with ", ", $(stringify!($element)),*) ," FROM ", $table_name);

            // By now, we fill all those not sortable values with id. That will be safe.
            const SORTABLE_COLUMNS: &'static [&'static str] = &[
                "id", $(if <$ty as crate::backend::database::DatabaseType>::IS_SORTABLE { stringify!($element) } else { "id" }),*
            ];

            type SelectValue<'a> = (i64, $( $ty ),*);

            fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> crate::backend::database::Record<Self> {
                let (primary_key, $( $element ),*) = value;
                crate::backend::database::Record {
                    identifier: crate::backend::database::PrimaryKey::from(primary_key),
                    value: $name { $( $element ),* }
                }
            }
        }

        impl crate::backend::database::SelectableByPrimaryKey for $name {
            const STATEMENT_SELECT: &'static str = const_format::concatcp!(<$name as crate::backend::database::Selectable>::STATEMENT_SELECT_ALL, " WHERE id = ?");

            /// Select an element and parse it.
            fn select(database: &crate::backend::database::Database, index: crate::backend::database::PrimaryKey<Self>) -> Result<Self::Output, crate::backend::database::Error> {
                Ok(database
                    .connection
                    .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                        Self::SelectValue::try_from(row).map(<Self as crate::backend::database::Selectable>::deserialize_sql)
                    })?)
            }

            /// Try to select a element which primary key was not validated.
            fn try_select(database: &crate::backend::database::Database, index: i64) -> Result<Option<Self::Output>, crate::backend::database::Error> {
                use rusqlite::OptionalExtension;

                Ok(database
                    .connection
                    .query_row(Self::STATEMENT_SELECT, (index,), |row| {
                        Self::SelectValue::try_from(row).map(<Self as crate::backend::database::Selectable>::deserialize_sql)
                    })
                    .optional()?)
            }
        }
    };
}

macro_rules! impl_referenceable {
    () => {};
    ($name: ident => $value: expr) => {
        impl crate::backend::database::Referenceable for $name {
            const STATEMENT_SELECT_NAME: &'static str = const_format::concatcp!(
                "SELECT id, ",
                $value,
                " FROM ",
                <$name as crate::backend::database::DatabaseEntry>::TABLE_NAME
            );
        }
    };
    ($name: ident => $value: expr; testing) => {
        // Primarily here for enabling this conditionally.
        const _DESCRIPTOR: &'static str = $value;

        #[test]
        fn test_select_primary_key_description_empty() {
            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let descriptions = $name::generate_descriptions(&database).expect("valid descriptions");
            assert_eq!(descriptions.len(), 0);
        }

        #[test]
        fn test_select_primary_key_description() {
            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let example = $name::create_default(&database);
            let record = example.insert_record(&database).expect("insert sucessfull");

            let descriptions = $name::generate_descriptions(&database).expect("valid descriptions");
            assert_eq!(descriptions.len(), 1);
            assert_eq!(descriptions[0].0, record.identifier);
        }
    };
}

#[cfg(test)]
macro_rules! impl_select_test {
    (true, $name: ident $(,$foreign_key_descriptor: expr)?) => {
        use crate::backend::database::{Referenceable, Selectable, SelectableByPrimaryKey};

        #[test]
        fn test_select_automatically() {
            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let example = $name::create_default(&database);
            let record = example.insert_record(&database).expect("insert sucessfull");
            let loaded_example = $name::select(&database, record.identifier).expect("valid sample");

            assert_eq!(<$name as Selectable>::Output::from(record), loaded_example)
        }

        #[test]
        fn test_select_all() {
            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let example = $name::create_default(&database);
            let record = example.insert_record(&database).expect("insert sucessfull");

            let loaded_examples = $name::select_all(&database).expect("valid sample");
            assert_eq!(loaded_examples.len(), 1);
            assert_eq!(
                <$name as Selectable>::Output::from(record),
                loaded_examples[0]
            )
        }

        #[test]
        fn test_select_raw() {
            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let example = $name::create_default(&database);
            let record = example.insert_record(&database).expect("insert sucessfull");

            let loaded_example =
                $name::try_select(&database, record.identifier.raw_index()).expect("valid sample");
            assert_eq!(
                <$name as Selectable>::Output::from(record),
                loaded_example.expect("existing element")
            )
        }

        #[test]
        fn test_select_raw_noexisting() {
            const NONEXISTING_INDEX: i64 = 42;

            let database = crate::backend::database::Database::plain().expect("valid database");
            $name::create_table(&database).expect("valid table");

            let index = $name::create_default(&database)
                .insert(&database)
                .expect("insert sucessfull");
            assert_ne!(index.0, NONEXISTING_INDEX);

            assert!($name::try_select(&database, NONEXISTING_INDEX)
                .expect("valid sample")
                .is_none());
        }
    };
    (false, $name: ident) => {};
}

/// Create a indexable database entry.
macro_rules! make_struct {(
    $(#[derive($( $derived: ty ),+)] )?
    #[table($table_name: expr)]
    #[dependencies( $dependencies: ty )]
    #[impl_select($should_impl: expr, testing: $should_impl_text: expr $(, description: $foreign_key_descriptor: expr)?)]
    $name: ident { $( $(#[$os_attr: meta])? $element: ident: $ty: ty),* } $( ($additional_conditions: expr) )?
) => {
    paste::paste! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        $(#[derive($( $derived ),+)])?
        pub struct $name {
            $( $(#[$os_attr])? pub $element: $ty),*
        }

        impl crate::backend::database::DatabaseEntry for $name {
            type DependsOn = $dependencies;

            const TABLE_NAME: &'static str = $table_name;
            const STATEMENT_CREATE_TABLE: &'static str = const_format::concatcp!(
                "CREATE TABLE IF NOT EXISTS ", $table_name, " (id INTEGER PRIMARY KEY", $( ", ", stringify!($element), " ", <$ty as crate::backend::database::DatabaseType>::COLUMN_VALUE ),* $(, ", ", $additional_conditions, " ")? , " )"
            );
        }

        impl crate::backend::database::Insertable for $name {
            const STATEMENT_INSERT: &'static str = std::concat!(
                "INSERT INTO ", $table_name, " (", concat_with::concat!(with ", ", $(stringify!($element)),*), ") VALUES (", concat_with::concat!(with ", ", $(crate::backend::database::question_mark!($element)),*), ")"
            );

            type InsertValue<'a> = ($( &'a $ty ),*, );

            fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a> {
                ($( &self.$element ),* ,)
            }
        }

        impl crate::backend::database::Indexable for $name { }

        crate::backend::database::impl_select!($should_impl, $name, $table_name, $($element: $ty),*);

        crate::backend::database::impl_referenceable!($(
            $name => $foreign_key_descriptor
        )?);

        #[cfg(test)]
        mod [< "test_" $table_name >] {
            use crate::backend::database::{DatabaseEntry, Insertable, DefaultGenerator};
            use super::$name;

            #[test]
            fn test_insert_automatically() {
                let database = crate::backend::database::Database::plain().expect("valid database");
                $name::create_table(&database).expect("valid table");
                $name::create_default(&database).insert(&database).expect("insert sucessfull");
            }

            crate::backend::database::impl_select_test!($should_impl_text, $name);

            crate::backend::database::impl_referenceable!($(
                $name => $foreign_key_descriptor; testing
            )?);
        }
    }
}}

pub(crate) use impl_referenceable;
pub(crate) use impl_select;
#[cfg(test)]
pub(crate) use impl_select_test;
pub(crate) use make_struct;
pub(crate) use question_mark;

#[cfg(test)]
mod test {
    use crate::backend::{
        database::{
            Database, DatabaseEntry, Insertable, PrimaryKey, Record, Selectable,
            SelectableByPrimaryKey,
        },
        Limit, Order, Pagination,
    };

    crate::backend::database::make_struct!(
        #[derive(Default, serde::Serialize, serde::Deserialize)]
        #[table("tests")]
        #[dependencies(())]
        #[impl_select(true, testing: true, description: "string_value")]
        Test {
            bool_value: bool,
            string_value: String,
            integer_value: u32
        }
    );

    // We need to check values with single elements are properly serialized, too.
    crate::backend::database::make_struct!(
        #[derive(Default)]
        #[table("tests_single")]
        #[dependencies(())]
        #[impl_select(true, testing: false)]
        TestSingleElement {
            string_value: String
        }
    );

    fn generate_pagination_data(pagination: Pagination<Test>) -> Vec<u32> {
        let database = Database::plain().expect("valid database");
        Test::create_table(&database).expect("valid table");

        // Fill the values
        for integer_value in [42u32, 43, 44] {
            Test {
                bool_value: false,
                string_value: String::from("ABC"),
                integer_value,
            }
            .insert(&database)
            .expect("insert sucessfull");
        }

        let created_values =
            Test::select_all_sorted(&database, pagination).expect("valid database query");

        created_values
            .iter()
            .map(|value| value.integer_value)
            .collect()
    }

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
            identifier: crate::backend::database::PrimaryKey::from(0),
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
            identifier: crate::backend::database::PrimaryKey::from(0),
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
    fn test_primary_key_serialization() {
        let primary_key: PrimaryKey<Test> = crate::backend::database::PrimaryKey::from(0);
        assert_eq!(
            serde_json::to_string(&primary_key).expect("str"),
            "\"/tests/0\""
        )
    }

    #[test]
    fn test_primary_key_deserialization() {
        const TEST_INPUT: &'static str = "\"/tests/42\"";
        assert_eq!(
            serde_json::from_str::<crate::backend::database::PrimaryKey<Test>>(TEST_INPUT)
                .expect("valid key"),
            crate::backend::database::PrimaryKey::from(42)
        );
    }

    #[test]
    fn test_sortable_columns() {
        assert_eq!(Test::SORTABLE_COLUMNS, ["id", "id", "id", "integer_value"]);
    }

    #[test]
    fn test_sorted_select_asc() {
        let pagination = Pagination::new("integer_value", 0, Limit::from(5), Order::Ascending)
            .expect("valid pagination");

        assert_eq!(generate_pagination_data(pagination), vec![42, 43, 44]);
    }

    #[test]
    fn test_sorted_select_desc() {
        let pagination = Pagination::new("integer_value", 0, Limit::from(5), Order::Descending)
            .expect("valid pagination");

        assert_eq!(generate_pagination_data(pagination), vec![44, 43, 42]);
    }

    #[test]
    fn test_sorted_select_desc_offset() {
        let pagination = Pagination::new("integer_value", 1, Limit::from(5), Order::Descending)
            .expect("valid pagination");

        assert_eq!(generate_pagination_data(pagination), vec![43, 42]);
    }

    #[test]
    fn test_sorted_select_desc_offset_and_limit() {
        let pagination = Pagination::new("integer_value", 0, Limit::from(2), Order::Descending)
            .expect("valid pagination");

        assert_eq!(generate_pagination_data(pagination), vec![44, 43]);
    }
}
