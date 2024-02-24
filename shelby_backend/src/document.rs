use crate::database::{DefaultGenerator, Insertable, PrimaryKey};
use crate::{person::Person, user::User, Date};

crate::database::make_struct!(
    #[derive(serde::Serialize, serde::Deserialize)]
    #[table("documents")]
    #[dependencies((Person, User))]
    Document {
        document: Vec<u8>,
        processed_by: PrimaryKey<User>,
        from_person: PrimaryKey<Person>,
        to_person: PrimaryKey<Person>,
        recieved: Date,
        processed: Date,
        description: Option<String>
    } ("FOREIGN KEY(processed_by) REFERENCES users(id), FOREIGN KEY(from_person) REFERENCES persons(id), FOREIGN KEY(to_person) REFERENCES persons(id)")
);

impl DefaultGenerator for Document {
    fn create_default(database: &crate::database::Database) -> Self {
        let person = Person::default().insert(&database).expect("valid person");
        let user = User::create_default(&database)
            .insert(&database)
            .expect("valid user");

        Document {
            document: Vec::default(),
            processed_by: user,
            from_person: person,
            to_person: person,
            recieved: Date::today(),
            processed: Date::today(),
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Document;
    use crate::database::{DefaultGenerator, Insertable};

    #[test]
    fn test_availability_in_default_migrations() {
        let database = crate::database::Database::in_memory().expect("valid database");
        Document::create_default(&database)
            .insert(&database)
            .expect("insert sucessfull");
    }
}
