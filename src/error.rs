use rocket::{http::Status, response::Responder};

#[derive(Debug, PartialEq)]
pub enum Error {
    DatabaseError(shelby_backend::Error),
    NotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::DatabaseError(error) => write!(f, "database error: {}", error),
            Error::NotFound => write!(f, "element not found"),
        }
    }
}

impl From<shelby_backend::Error> for Error {
    fn from(value: shelby_backend::Error) -> Self {
        Error::DatabaseError(value)
    }
}

impl std::error::Error for Error {}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        // ToDo: Log here
        match self {
            Error::DatabaseError(_) => Status::InternalServerError.respond_to(request),
            Error::NotFound => Status::NotFound.respond_to(request),
        }
    }
}
