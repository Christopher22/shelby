use chrono::{DateTime, Utc};

use crate::{person::Person, user::User, PrimaryKey};

crate::macros::make_struct!(
    Document (Table: "documents") depends on (Person, User)  => {
        document: Vec<u8> => "BLOB NOT NULL",
        processed_by: PrimaryKey<User> => "INTEGER NOT NULL",
        from_person: PrimaryKey<Person> => "INTEGER NOT NULL",
        to_person: PrimaryKey<Person> => "INTEGER NOT NULL",
        recieved: DateTime<Utc> => "DATETIME NOT NULL",
        processed: DateTime<Utc> => "DATETIME NOT NULL",
        description: Option<String> => "STRING"
    } ("FOREIGN KEY(processed_by) REFERENCES users(id), FOREIGN KEY(from_person) REFERENCES persons(id), FOREIGN KEY(to_person) REFERENCES persons(id)")
);

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::Document;
    use crate::{person::Person, user::User, IndexableDatebaseEntry, TestGenerator};

    impl crate::TestGenerator for Document {
        fn create_example(database: &crate::Database) -> Self {
            let person = Person::default().insert(&database).expect("valid person");
            let user = User::default().insert(&database).expect("valid user");

            Document {
                document: Vec::default(),
                processed_by: user,
                from_person: person,
                to_person: person,
                recieved: DateTime::default(),
                processed: DateTime::default(),
                description: None,
            }
        }
    }

    #[test]
    fn test_availability_in_default_migrations() {
        let database = crate::Database::in_memory().expect("valid database");
        Document::create_example(&database)
            .insert(&database)
            .expect("insert sucessfull");
    }
}
