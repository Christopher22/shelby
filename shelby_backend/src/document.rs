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
    }
);
