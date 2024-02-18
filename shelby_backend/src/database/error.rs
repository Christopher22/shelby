/// An error issued by the underlying database.
#[derive(Debug, PartialEq)]
pub struct Error(rusqlite::Error);

impl Error {
    /// Allow the check if the error results from an invalid jet specified foreign key.
    pub fn is_constraint_violation(&self) -> bool {
        match &self.0 {
            rusqlite::Error::SqliteFailure(error, _) => {
                println!("{}", error);
                error.code == rusqlite::ffi::ErrorCode::ConstraintViolation
            }
            _ => false,
        }
    }
}

// ToDo: Check forign key constraint
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

#[cfg(test)]
mod tests {
    use crate::{
        database::{Database, IndexableDatebaseEntry, PrimaryKey},
        user::User,
    };

    #[test]
    fn test_foreign_key_error() {
        let database = Database::in_memory().expect("valid database");
        let document = User {
            related_to: Some(PrimaryKey::from(42)),
            ..User::default()
        };

        assert!(document
            .insert(&database)
            .expect_err("insertion despite invalid foreign key")
            .is_constraint_violation())
    }
}
