use rusqlite::{Connection, Result};

pub struct Database {
    pub(crate) connection: Connection,
}

impl Database {
    pub fn from_memory() -> Result<Self, rusqlite::Error> {
        Connection::open_in_memory().map(|connection| Database { connection })
    }
}
