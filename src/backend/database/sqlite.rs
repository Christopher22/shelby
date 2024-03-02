use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use super::{DatabaseEntry, Error};

pub struct Database {
    pub(crate) connection: Connection,
}

impl Database {
    /// Open the database in memory.
    pub fn in_memory() -> Result<Self, Error> {
        Ok(Connection::open_in_memory().and_then(|connection| {
            let mut database = Database { connection };
            database.prepare_connection()?;
            Ok(database)
        })?)
    }

    /// Get a raw SQLite database. This should only be relevant for unit testing purposes.
    #[cfg(test)]
    pub fn plain() -> Result<Self, Error> {
        Ok(Connection::open_in_memory().map(|connection| Database { connection })?)
    }

    fn prepare_connection(&mut self) -> Result<(), rusqlite::Error> {
        Self::get_migrations()
            .to_latest(&mut self.connection)
            .map_err(|error| match error {
                rusqlite_migration::Error::RusqliteError { query: _, err } => err,
                _ => panic!("Unexpected error in running the migration"),
            })?;

        self.connection.pragma_update(None, "journal_mode", "WAL")?;
        self.connection.pragma_update(None, "foreign_keys", "ON")
    }

    fn get_migrations() -> Migrations<'static> {
        Migrations::new(vec![M::up(const_format::concatcp!(
            crate::backend::person::Person::STATEMENT_CREATE_TABLE,
            "; ",
            crate::backend::person::Group::STATEMENT_CREATE_TABLE,
            "; ",
            crate::backend::person::Membership::STATEMENT_CREATE_TABLE,
            "; ",
            crate::backend::user::User::STATEMENT_CREATE_TABLE,
            "; ",
            crate::backend::document::Document::STATEMENT_CREATE_TABLE,
            "; ",
        ))
        .down(const_format::concatcp!(
            "DROP TABLE ",
            crate::backend::person::Person::TABLE_NAME,
            "; DROP TABLE ",
            crate::backend::person::Group::TABLE_NAME,
            "; DROP TABLE ",
            crate::backend::person::Membership::TABLE_NAME,
            "; DROP TABLE ",
            crate::backend::user::User::TABLE_NAME,
            "; ",
        ))])
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::database::Database;

    #[test]
    fn test_migrations() {
        assert!(Database::get_migrations().validate().is_ok());
    }
}
