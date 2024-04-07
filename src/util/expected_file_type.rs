use rocket::http::{MediaType, Status};
use rocket::request::{self, FromRequest, Outcome, Request};

pub trait FileType: Default {
    fn is_suitable(media_type: &MediaType) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Html;

impl FileType for Html {
    fn is_suitable(media_type: &MediaType) -> bool {
        media_type.is_html()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Json;

impl FileType for Json {
    fn is_suitable(media_type: &MediaType) -> bool {
        media_type.is_json() || media_type.is_json_api()
    }
}

/// A easy guard to differentiate human-readable from JSON routes.
pub struct ExpectedFileType<T>(T);

#[rocket::async_trait]
impl<'r, T: FileType> FromRequest<'r> for ExpectedFileType<T> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.format() {
            Some(value) if value.is_html() => Outcome::Success(Self(T::default())),
            _ => Outcome::Forward(Status::NotAcceptable),
        }
    }
}
