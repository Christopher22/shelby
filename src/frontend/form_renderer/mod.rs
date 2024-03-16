mod field;
mod insertable_database_entry;
mod util;

use std::collections::HashMap;

use rocket::serde::ser::SerializeStruct;
use rocket::serde::{Serialize, Serializer};

pub use self::field::*;
pub use self::insertable_database_entry::*;
pub use self::util::*;

use super::Renderable;
use crate::auth::AuthenticatedUser;
use crate::backend::database::{Database, Referenceable};
use crate::util::FormInputType;

pub struct InsertFormRenderer<'a, T> {
    post_url: &'static str,
    database: &'a Database,
    user: AuthenticatedUser,
    marker: std::marker::PhantomData<*const T>,
}

impl<'a, T> InsertFormRenderer<'a, T> {
    fn new(post_url: &'static str, database: &'a Database, user: AuthenticatedUser) -> Self {
        Self {
            post_url,
            database,
            user,
            marker: std::marker::PhantomData,
        }
    }
}

impl<'a, const N: usize, T: InsertableDatabaseEntry<FieldsType = [Field; N]>> Renderable
    for InsertFormRenderer<'a, T>
where
    [Field; N]: Clone + Serialize,
{
    const TEMPLATE: &'static str = "form";

    fn generate_context(&self) -> impl rocket::serde::Serialize {
        let mut foreign_key_storage = ForeignKeyStorage::from(self.database);
        let mut fields = T::FIELDS.clone();
        for field in fields.iter_mut() {
            match &mut field.input_type {
                InputType::Hidden(hidden) => hidden.set_value(&self.user),
                InputType::ForeignKey(foreign_key) => foreign_key
                    .load(&mut foreign_key_storage)
                    .unwrap_or_default(),
                _ => {}
            };
        }

        rocket_dyn_templates::context! {
            name: &T::NAME,
            fields: fields,
            post_url: self.post_url,
            method: T::PostMethod::DATA_TYPE,
            foreign_keys: foreign_key_storage
        }
    }
}
