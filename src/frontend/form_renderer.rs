use rocket::serde::Serialize;
use serde::ser::SerializeStruct;

use super::Renderable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Text(Metadata),
    Email(Metadata),
    Password(Metadata),
    Date(Metadata),
    Hidden(fn() -> String),
}

impl InputType {
    const fn html_value(&self) -> &'static str {
        match self {
            InputType::Text(_) => "text",
            InputType::Password(_) => "password",
            InputType::Email(_) => "email",
            InputType::Date(_) => "date",
            InputType::Hidden(_) => "hidden",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    label: &'static str,
    placeholder: Option<&'static str>,
    required: bool,
}

#[derive(Debug)]
pub struct Field {
    id: &'static str,
    input_type: InputType,
    attributes: &'static [&'static str],
}

impl Serialize for Field {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut result = match self.input_type {
            InputType::Text(meta)
            | InputType::Email(meta)
            | InputType::Date(meta)
            | InputType::Password(meta) => {
                let mut result = serializer.serialize_struct("Field", 6)?;
                result.serialize_field("required", &meta.required)?;
                result.serialize_field("placeholder", &meta.placeholder)?;
                result.serialize_field("label", &meta.label)?;
                result
            }
            InputType::Hidden(value_generator) => {
                let mut result = serializer.serialize_struct("Field", 4)?;
                result.serialize_field("value", &value_generator())?;
                result
            }
        };

        result.serialize_field("name", self.id)?;
        result.serialize_field("input_type", self.input_type.html_value())?;
        result.serialize_field("attributes", &self.attributes)?;
        result.end()
    }
}

pub trait InsertableDatabaseEntry<const N: usize>: Sized {
    const NAME: &'static str;
    const FIELDS: [Field; N];

    fn prepare_rendering(post_url: &'static str) -> InsertFormRenderer<N, Self> {
        InsertFormRenderer::new(post_url)
    }
}

pub struct InsertFormRenderer<const N: usize, T>(
    &'static str,
    std::marker::PhantomData<*const [T; N]>,
);

impl<const N: usize, T> InsertFormRenderer<N, T> {
    fn new(post_url: &'static str) -> Self {
        Self(post_url, std::marker::PhantomData)
    }
}

impl<const N: usize, T: InsertableDatabaseEntry<N>> Renderable for InsertFormRenderer<N, T>
where
    [Field; N]: Serialize,
{
    const TEMPLATE: &'static str = "form";

    fn generate_context(&self) -> impl rocket::serde::Serialize {
        rocket_dyn_templates::context! {
            name: &T::NAME,
            fields: &T::FIELDS,
            post_url: self.0
        }
    }
}

impl InsertableDatabaseEntry<5> for shelby_backend::person::Person {
    const NAME: &'static str = "New person";
    const FIELDS: [Field; 5] = [
        Field {
            id: "name",
            input_type: InputType::Text(Metadata {
                label: "Name",
                placeholder: Some("Full name of the person"),
                required: true,
            }),
            attributes: &[],
        },
        Field {
            id: "address",
            input_type: InputType::Text(Metadata {
                label: "Address",
                placeholder: Some("Address of the person"),
                required: true,
            }),
            attributes: &[],
        },
        Field {
            id: "email",
            input_type: InputType::Email(Metadata {
                label: "E-Mail",
                placeholder: Some("E-mail of the person"),
                required: true,
            }),
            attributes: &[],
        },
        Field {
            id: "birthday",
            input_type: InputType::Date(Metadata {
                label: "Birthday",
                placeholder: Some("Birthday of the person"),
                required: false,
            }),
            attributes: &[],
        },
        Field {
            id: "comment",
            input_type: InputType::Text(Metadata {
                label: "Comment",
                placeholder: Some("More comments regarding the person"),
                required: false,
            }),
            attributes: &[],
        },
    ];
}

impl InsertableDatabaseEntry<1> for shelby_backend::document::Document {
    const NAME: &'static str = "New document";
    const FIELDS: [Field; 1] = [Field {
        id: "name",
        input_type: InputType::Text(Metadata {
            label: "Name",
            placeholder: Some("The name of the document"),
            required: true,
        }),
        attributes: &[],
    }];
}

impl InsertableDatabaseEntry<1> for shelby_backend::person::Group {
    const NAME: &'static str = "New group";
    const FIELDS: [Field; 1] = [Field {
        id: "description",
        input_type: InputType::Text(Metadata {
            label: "Description",
            placeholder: Some("Description of the new group"),
            required: true,
        }),
        attributes: &[],
    }];
}

impl InsertableDatabaseEntry<5> for shelby_backend::user::User {
    const NAME: &'static str = "New user";
    const FIELDS: [Field; 5] = [
        Field {
            id: "username",
            input_type: InputType::Text(Metadata {
                label: "User name",
                placeholder: Some("Name of the new user"),
                required: true,
            }),
            attributes: &[],
        },
        Field {
            id: "password_hash",
            input_type: InputType::Password(Metadata {
                label: "Password",
                placeholder: Some("Password of the new user"),
                required: true,
            }),
            attributes: &[],
        },
        Field {
            id: "related_to",
            input_type: InputType::Text(Metadata {
                label: "Primary user",
                placeholder: Some("ID of the primary user"),
                required: false,
            }),
            attributes: &[],
        },
        Field {
            id: "creation_date",
            input_type: InputType::Hidden(|| shelby_backend::Date::today().to_string()),
            attributes: &[],
        },
        Field {
            id: "active",
            input_type: InputType::Hidden(|| String::from("true")),
            attributes: &[],
        },
    ];
}
