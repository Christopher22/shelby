use crate::backend::person::Person;
use crate::util::FormInputType;
use crate::{auth::AuthenticatedUser, backend::database::Database};

use super::{Field, FileMetadata, InputType, InsertFormRenderer, Metadata};

/// A database entry which might be inserted over a form.
pub trait InsertableDatabaseEntry: Sized {
    /// The method hwo data is send to the webserver.
    type PostMethod: for<'a> FormInputType<'a>;
    /// The type for storing the fields. Should be an array of fields. However, this avoids generic arguments in the trait definition.
    type FieldsType;

    const NAME: &'static str;
    const FIELDS: Self::FieldsType;

    fn prepare_rendering<'a>(
        post_url: &'static str,
        database: &'a Database,
        user: AuthenticatedUser,
    ) -> InsertFormRenderer<'a, Self> {
        InsertFormRenderer::new(post_url, database, user)
    }
}

impl InsertableDatabaseEntry for crate::backend::person::Person {
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

impl InsertableDatabaseEntry for crate::backend::document::Document {
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
            InputType::new_foreign::<Person>(Metadata {
                label: "From",
                placeholder: Some("ID of the sending person"),
                required: true,
            }),
        ),
        Field::new(
            "to_person",
            InputType::new_foreign::<Person>(Metadata {
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
            InputType::new_hidden(|user| user.user.to_string()),
        ),
        Field::new(
            "processed",
            InputType::new_hidden(|_| crate::backend::Date::today().to_string()),
        ),
    ];

    type PostMethod = crate::util::FlexibleInput<Self>;
    type FieldsType = [Field; 7];
}

impl InsertableDatabaseEntry for crate::backend::person::Group {
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

impl InsertableDatabaseEntry for crate::backend::user::User {
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
            InputType::new_hidden(|_| crate::backend::Date::today().to_string()),
        ),
        Field::new("active", InputType::new_hidden(|_| String::from("true"))),
    ];

    type PostMethod = rocket::serde::json::Json<Self>;
    type FieldsType = [Field; 5];
}