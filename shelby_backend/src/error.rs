/// An error issued by the underlying database.
#[derive(Debug, PartialEq)]
pub struct Error(rusqlite::Error);

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL error: {}", &self.0)
    }
}

impl std::error::Error for Error {}
