use rocket::serde::ser::SerializeStruct;
use rocket::serde::{Serialize, Serializer};

use super::Renderable;
use crate::util::FormInputType;

type Context = crate::auth::AuthenticatedUser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Text(Metadata),
    Email(Metadata),
    Password(Metadata),
    Date(Metadata),
    File(FileMetadata),
    Hidden(fn(&Context) -> String),
}

impl InputType {
    const fn html_value(&self) -> &'static str {
        match self {
            InputType::Text(_) => "text",
            InputType::Password(_) => "password",
            InputType::Email(_) => "email",
            InputType::Date(_) => "date",
            InputType::Hidden(_) => "hidden",
            InputType::File(_) => "file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    label: &'static str,
    placeholder: Option<&'static str>,
    required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    label: &'static str,
    extensions: &'static [&'static str],
    multiple: bool,
}

#[derive(Debug)]
pub struct Field<T = ()> {
    id: &'static str,
    input_type: InputType,
    attributes: &'static [&'static str],
    context: T,
}

impl Field<()> {
    pub const fn new(id: &'static str, input_type: InputType) -> Self {
        Field {
            id,
            input_type,
            attributes: &[],
            context: (),
        }
    }

    pub const fn set_context<C>(self, context: C) -> Field<C> {
        Field {
            id: self.id,
            input_type: self.input_type,
            attributes: self.attributes,
            context,
        }
    }
}

impl<'a> Serialize for Field<&'a Context> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const NUM_GENERAL_ELEMENTS: usize = 3;

        let mut result = match self.input_type {
            InputType::Text(meta)
            | InputType::Email(meta)
            | InputType::Date(meta)
            | InputType::Password(meta) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 3)?;
                result.serialize_field("required", &meta.required)?;
                result.serialize_field("placeholder", &meta.placeholder)?;
                result.serialize_field("label", &meta.label)?;
                result
            }
            InputType::Hidden(value_generator) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 1)?;
                result.serialize_field("value", &value_generator(&self.context))?;
                result
            }
            InputType::File(meta_data) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 3)?;
                result.serialize_field("accept", meta_data.extensions)?;
                result.serialize_field("label", meta_data.label)?;
                result.serialize_field("multiple", &meta_data.multiple)?;
                result
            }
        };

        result.serialize_field("name", self.id)?;
        result.serialize_field("input_type", self.input_type.html_value())?;
        result.serialize_field("attributes", &self.attributes)?;
        result.end()
    }
}

/// A database entry which might be inserted over a form.
pub trait InsertableDatabaseEntry: Sized {
    /// The method hwo data is send to the webserver.
    type PostMethod: for<'a> FormInputType<'a>;
    /// The type for storing the fields. Should be an array of fields. However, this avoids generic arguments in the trait definition.
    type FieldsType;

    const NAME: &'static str;
    const FIELDS: Self::FieldsType;

    fn prepare_rendering<C>(post_url: &'static str, context: C) -> InsertFormRenderer<Self, C> {
        InsertFormRenderer::new(post_url, context)
    }
}

pub struct InsertFormRenderer<T, C>(&'static str, C, std::marker::PhantomData<*const T>);

impl<T, C> InsertFormRenderer<T, C> {
    fn new(post_url: &'static str, context: C) -> Self {
        Self(post_url, context, std::marker::PhantomData)
    }
}

impl<const N: usize, T: InsertableDatabaseEntry<FieldsType = [Field; N]>, C> Renderable
    for InsertFormRenderer<T, C>
where
    for<'a> [Field<&'a C>; N]: Serialize,
{
    const TEMPLATE: &'static str = "form";

    fn generate_context(&self) -> impl rocket::serde::Serialize {
        let fields_with_context = T::FIELDS.map(|value| value.set_context(&self.1));
        rocket_dyn_templates::context! {
            name: &T::NAME,
            fields: fields_with_context,
            post_url: self.0,
            method: T::PostMethod::DATA_TYPE
        }
    }
}

impl InsertableDatabaseEntry for shelby_backend::person::Person {
    const NAME: &'static str = "New person";
    const FIELDS: [Field; 5] = [
        Field::new(
            "name",
            InputType::Text(Metadata {
                label: "Name",
                placeholder: Some("Full name of the person"),
                required: true,
            }),
        ),
        Field::new(
            "address",
            InputType::Text(Metadata {
                label: "Address",
                placeholder: Some("Address of the person"),
                required: true,
            }),
        ),
        Field::new(
            "email",
            InputType::Email(Metadata {
                label: "E-Mail",
                placeholder: Some("E-mail of the person"),
                required: true,
            }),
        ),
        Field::new(
            "birthday",
            InputType::Date(Metadata {
                label: "Birthday",
                placeholder: Some("Birthday of the person"),
                required: false,
            }),
        ),
        Field::new(
            "comment",
            InputType::Text(Metadata {
                label: "Comment",
                placeholder: Some("More comments regarding the person"),
                required: false,
            }),
        ),
    ];

    type PostMethod = rocket::serde::json::Json<Self>;
    type FieldsType = [Field; 5];
}

impl InsertableDatabaseEntry for shelby_backend::document::Document {
    const NAME: &'static str = "New document";
    const FIELDS: [Field; 7] = [
        Field::new(
            "document",
            InputType::File(FileMetadata {
                label: "File",
                extensions: &[".pdf"],
                multiple: false,
            }),
        ),
        Field::new(
            "from_person",
            InputType::Text(Metadata {
                label: "From",
                placeholder: Some("ID of the sending person"),
                required: true,
            }),
        ),
        Field::new(
            "to_person",
            InputType::Text(Metadata {
                label: "To",
                placeholder: Some("ID of the recieving person"),
                required: true,
            }),
        ),
        Field::new(
            "recieved",
            InputType::Date(Metadata {
                label: "Recieved",
                placeholder: Some("The date the document was recieved"),
                required: true,
            }),
        ),
        Field::new(
            "description",
            InputType::Text(Metadata {
                label: "Description",
                placeholder: Some("The description of the document"),
                required: false,
            }),
        ),
        // Private field from here
        Field::new(
            "processed_by",
            InputType::Hidden(|user| user.user.to_string()),
        ),
        Field::new(
            "processed",
            InputType::Hidden(|_| shelby_backend::Date::today().to_string()),
        ),
    ];

    type PostMethod = crate::util::FlexibleInput<Self>;
    type FieldsType = [Field; 7];
}

impl InsertableDatabaseEntry for shelby_backend::person::Group {
    const NAME: &'static str = "New group";
    const FIELDS: [Field; 1] = [Field::new(
        "description",
        InputType::Text(Metadata {
            label: "Description",
            placeholder: Some("Description of the new group"),
            required: true,
        }),
    )];

    type PostMethod = rocket::serde::json::Json<Self>;
    type FieldsType = [Field; 1];
}

impl InsertableDatabaseEntry for shelby_backend::user::User {
    const NAME: &'static str = "New user";
    const FIELDS: [Field; 5] = [
        Field::new(
            "username",
            InputType::Text(Metadata {
                label: "User name",
                placeholder: Some("Name of the new user"),
                required: true,
            }),
        ),
        Field::new(
            "password",
            InputType::Password(Metadata {
                label: "Password",
                placeholder: Some("Password of the new user"),
                required: true,
            }),
        ),
        Field::new(
            "related_to",
            InputType::Text(Metadata {
                label: "Primary user",
                placeholder: Some("ID of the primary user"),
                required: false,
            }),
        ),
        Field::new(
            "creation_date",
            InputType::Hidden(|_| shelby_backend::Date::today().to_string()),
        ),
        Field::new("active", InputType::Hidden(|_| String::from("true"))),
    ];

    type PostMethod = rocket::serde::json::Json<Self>;
    type FieldsType = [Field; 5];
}
