use rocket::{
    async_trait,
    data::{FromData, Outcome},
    form::{Form, FromForm},
    http::Status,
    serde::json::Json,
    Data, Request,
};
use shelby_backend::{database::PrimaryKey, document::Document, Date};

/// Data which might be parsted both as JSON and as a form.
pub struct FlexibleInput<T: MappableForm>(T);

impl<T: MappableForm> FlexibleInput<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

pub trait MappableForm: Sized + for<'a> rocket::serde::Deserialize<'a> {
    type FormType<'r>: FromForm<'r>;

    fn try_map<'r>(form: Self::FormType<'r>) -> Result<Self, (Status, MappingError<'r>)>;
}

#[async_trait]
impl<'r, T: MappableForm> FromData<'r> for FlexibleInput<T>
where
    T::FormType<'r>: FromForm<'r>,
{
    type Error = MappingError<'r>;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        match req.format() {
            Some(value) if value.is_json() => Json::<T>::from_data(req, data)
                .await
                .map(|value| FlexibleInput(value.into_inner().into()))
                .map_error(|(status, error)| (status, MappingError::JsonError(error))),
            Some(value) if value.is_form() || value.is_form_data() => {
                // Try to map the form
                Form::<T::FormType<'r>>::from_data(req, data)
                    .await
                    .map_error(|(status, error)| (status, MappingError::FormError(error)))
                    .and_then(|value| match T::try_map(value.into_inner()) {
                        Ok(value) => Outcome::Success(FlexibleInput(value)),
                        Err(error) => Outcome::Error(error),
                    })
            }
            _ => Outcome::Error((
                Status::UnsupportedMediaType,
                MappingError::MappingError("value neither json nor form"),
            )),
        }
    }
}

#[derive(Debug)]
pub enum MappingError<'r> {
    JsonError(rocket::serde::json::Error<'r>),
    FormError(rocket::form::error::Errors<'r>),
    MappingError(&'r str),
}

impl<'r> std::fmt::Display for MappingError<'r> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::JsonError(error) => write!(f, "{}", error),
            Self::FormError(error) => write!(f, "{}", error),
            Self::MappingError(error) => write!(f, "{}", error),
        }
    }
}

impl<'r> std::error::Error for MappingError<'r> {}

#[derive(FromForm)]
pub struct DocumentForm<'r> {
    document: &'r [u8],
    processed_by: i64,
    from_person: i64,
    to_person: i64,
    recieved: String,
    processed: String,
    description: Option<String>,
}

impl MappableForm for shelby_backend::document::Document {
    type FormType<'r> = DocumentForm<'r>;

    fn try_map<'r>(form: Self::FormType<'r>) -> Result<Self, (Status, MappingError<'r>)> {
        let recieved = Date::try_from(form.recieved.as_str()).or(Err((
            Status::BadRequest,
            MappingError::MappingError("'recieved' is not a valid date'"),
        )))?;

        let processed = Date::try_from(form.processed.as_str()).or(Err((
            Status::BadRequest,
            MappingError::MappingError("'processed' is not a valid date'"),
        )))?;

        Ok(Document {
            document: Vec::from(form.document),
            processed_by: PrimaryKey::from(form.processed_by),
            from_person: PrimaryKey::from(form.from_person),
            to_person: PrimaryKey::from(form.to_person),
            recieved,
            processed,
            description: form.description,
        })
    }
}
