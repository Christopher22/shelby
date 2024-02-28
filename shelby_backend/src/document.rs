use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::database::{Database, DatabaseEntry, DefaultGenerator, Insertable, PrimaryKey, Record};
use crate::{person::Person, user::User, Date};

crate::database::make_struct!(
    #[derive(serde::Serialize, serde::Deserialize)]
    #[table("documents")]
    #[dependencies((Person, User))]
    #[impl_select(false, testing: true)]
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

impl Document {
    /// Extract the document and store it in memory.
    pub fn load_into_memory(
        database: &Database,
        identifier: PrimaryKey<Self>,
    ) -> Result<Vec<u8>, crate::database::Error> {
        let mut blob = database
            .connection
            .blob_open(
                rusqlite::DatabaseName::Main,
                Document::TABLE_NAME,
                "document",
                identifier.raw_index(),
                true,
            )
            .map_err(crate::database::Error::from)?;

        let mut container = Vec::with_capacity(blob.size() as usize);
        blob.read_to_end(&mut container)
            .expect("reading blobs into allocated vector should not fail");
        Ok(container)
    }
}
impl crate::database::Selectable for Document {
    /// The public output. Other than the value itself, this value should be renderable in JSON without leaking sensible information.
    type Output = Metadata;

    /// The value which should be extracted from the row.
    type SelectValue<'a> = (
        PrimaryKey<Document>,
        PrimaryKey<User>,
        PrimaryKey<Person>,
        PrimaryKey<Person>,
        Date,
        Date,
        Option<String>,
    );

    /// The statement for selecting all entries.
    const STATEMENT_SELECT_ALL: &'static str = "SELECT id, processed_by, from_person, to_person, recieved, processed, description FROM documents";

    /// Deserialize the database value into a Record.
    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> Self::Output {
        Metadata {
            identifier: value.0,
            processed_by: value.1,
            from_person: value.2,
            to_person: value.3,
            recieved: value.4,
            processed: value.5,
            description: value.6,
        }
    }
}

impl crate::database::SelectableByPrimaryKey for Document {
    const STATEMENT_SELECT: &'static str = const_format::concatcp!(
        <Document as crate::database::Selectable>::STATEMENT_SELECT_ALL,
        " WHERE id = ?"
    );
}

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

/// The metadata of a document without the (large) document itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub identifier: PrimaryKey<Document>,
    pub processed_by: PrimaryKey<User>,
    pub from_person: PrimaryKey<Person>,
    pub to_person: PrimaryKey<Person>,
    pub recieved: Date,
    pub processed: Date,
    pub description: Option<String>,
}

impl From<Record<Document>> for Metadata {
    fn from(value: Record<Document>) -> Self {
        let identifier = value.identifier;
        let value: Document = value.value;
        Metadata {
            identifier,
            processed_by: value.processed_by,
            from_person: value.from_person,
            to_person: value.to_person,
            recieved: value.recieved,
            processed: value.processed,
            description: value.description,
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
