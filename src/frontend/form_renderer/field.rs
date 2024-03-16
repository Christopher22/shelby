use rocket::serde::ser::SerializeStruct;
use rocket::serde::{Serialize, Serializer};

use super::ForeignKeyStorage;
use crate::auth::AuthenticatedUser;
use crate::backend::database::Referenceable;

pub type HiddenCallback = fn(&AuthenticatedUser) -> String;

#[derive(Debug, Clone)]
pub struct Field {
    pub id: &'static str,
    pub input_type: InputType,
    pub attributes: &'static [&'static str],
}

impl Field {
    pub const fn new(id: &'static str, input_type: InputType) -> Self {
        Field {
            id,
            input_type,
            attributes: &[],
        }
    }
}

impl<'a, 'b> Serialize for Field {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        const NUM_GENERAL_ELEMENTS: usize = 3;

        let mut result = match &self.input_type {
            InputType::Text(meta)
            | InputType::Number(meta)
            | InputType::Email(meta)
            | InputType::Date(meta)
            | InputType::Password(meta) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 3)?;
                result.serialize_field("required", &meta.required)?;
                result.serialize_field("placeholder", &meta.placeholder)?;
                result.serialize_field("label", &meta.label)?;
                result
            }
            InputType::ForeignKey(foreign_key) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 4)?;
                result.serialize_field("required", &foreign_key.metadata.required)?;
                result.serialize_field("placeholder", &foreign_key.metadata.placeholder)?;
                result.serialize_field("label", &foreign_key.metadata.label)?;
                result.serialize_field("foreign_keys", foreign_key.list_name())?;
                result
            }
            InputType::Hidden(hidden_value) => {
                let mut result = serializer.serialize_struct("Field", NUM_GENERAL_ELEMENTS + 1)?;
                result.serialize_field(
                    "value",
                    &hidden_value.value.as_ref().expect("value previously set"),
                )?;
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

#[derive(Debug, Clone)]
pub enum InputType {
    Text(Metadata),
    Email(Metadata),
    Number(Metadata),
    Password(Metadata),
    Date(Metadata),
    File(FileMetadata),
    Hidden(HiddenValue),
    ForeignKey(ForeignKeyMetaData),
}

impl InputType {
    /// Shortcut for creating a new hidden item.
    pub const fn new_hidden(callback: HiddenCallback) -> Self {
        InputType::Hidden(HiddenValue::new(callback))
    }

    /// Shortcut for creating a new foreign key.
    pub const fn new_foreign<T: Referenceable>(metadata: Metadata) -> Self {
        InputType::ForeignKey(ForeignKeyMetaData::new::<T>(metadata))
    }

    const fn html_value(&self) -> &'static str {
        match self {
            InputType::Text(_) => "text",
            InputType::Number(_) => "number",
            InputType::ForeignKey(_) => "select",
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
    pub label: &'static str,
    pub placeholder: Option<&'static str>,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMetadata {
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    pub multiple: bool,
}

#[derive(Debug)]
pub struct HiddenValue {
    generator: HiddenCallback,
    pub value: Option<String>,
}

impl HiddenValue {
    pub const fn new(generator: HiddenCallback) -> Self {
        Self {
            generator,
            value: None,
        }
    }

    pub fn set_value(&mut self, user: &AuthenticatedUser) {
        self.value = Some((self.generator)(user));
    }
}

impl Clone for HiddenValue {
    fn clone(&self) -> HiddenValue {
        Self {
            generator: self.generator,
            value: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ForeignKeyMetaData {
    pub metadata: Metadata,
    callback: fn(&mut ForeignKeyStorage<'_>) -> Result<(), crate::backend::database::Error>,
    list_name: &'static str,
}

impl ForeignKeyMetaData {
    pub const fn new<T: Referenceable>(metadata: Metadata) -> Self {
        Self {
            metadata,
            callback: |value: &mut ForeignKeyStorage<'_>| value.add::<T>(),
            list_name: T::TABLE_NAME,
        }
    }

    pub fn load(
        &self,
        foreign_keys: &mut ForeignKeyStorage<'_>,
    ) -> Result<(), crate::backend::database::Error> {
        (self.callback)(foreign_keys)
    }

    pub fn list_name(&self) -> &'static str {
        self.list_name
    }
}
