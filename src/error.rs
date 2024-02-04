use rocket::{http::Status, response::Responder};

#[derive(Debug, PartialEq)]
pub enum Error {
    DatabaseError(shelby_backend::Error),
    NotFound,
    ConstraintViolation,
    WrongPassword
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::DatabaseError(error) => write!(f, "database error: {}", error),
            Error::NotFound => write!(f, "element not found"),
            Error::ConstraintViolation => write!(f, "invalid value"),
            Error::WrongPassword => write!(f, "invalid password"),
        }
    }
}

impl From<shelby_backend::Error> for Error {
    fn from(value: shelby_backend::Error) -> Self {
        match value.is_constraint_violation() {
            true => Error::ConstraintViolation,
            false => Error::DatabaseError(value),
        }
    }
}

impl std::error::Error for Error {}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        match self {
            Error::ConstraintViolation => Status::BadRequest.respond_to(request),
            Error::DatabaseError(database_error) => {
                eprintln!("{}", database_error);
                Status::InternalServerError.respond_to(request)
            },
            Error::NotFound => Status::NotFound.respond_to(request),
            Error::WrongPassword => Status::Unauthorized.respond_to(request),
        }
    }
}
