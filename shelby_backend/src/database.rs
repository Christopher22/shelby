use crate::DatabaseEntry;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub struct Database {
    pub(crate) connection: Connection,
}

impl Database {
    /// Open the database in memory.
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        Connection::open_in_memory().and_then(|connection| {
            let mut database = Database { connection };
            database.prepare_connection()?;
            Ok(database)
        })
    }

    /// Get a raw SQLite database. This should only be relevant for unit testing purposes.
    #[cfg(test)]
    pub fn plain() -> Result<Self, rusqlite::Error> {
        Connection::open_in_memory().map(|connection| Database { connection })
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
            crate::person::Person::STATEMENT_CREATE_TABLE,
            "; ",
            crate::person::Group::STATEMENT_CREATE_TABLE,
            "; ",
            crate::person::Membership::STATEMENT_CREATE_TABLE,
            "; ",
            crate::user::User::STATEMENT_CREATE_TABLE,
            "; ",
        ))
        .down(const_format::concatcp!(
            "DROP TABLE ",
            crate::person::Person::TABLE_NAME,
            "; DROP TABLE ",
            crate::person::Group::TABLE_NAME,
            "; DROP TABLE ",
            crate::person::Membership::TABLE_NAME,
            "; DROP TABLE ",
            crate::user::User::TABLE_NAME,
            "; ",
        ))])
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;

    #[test]
    fn test_migrations() {
        assert!(Database::get_migrations().validate().is_ok());
    }
}
