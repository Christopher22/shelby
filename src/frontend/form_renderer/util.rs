use std::collections::HashMap;

use rocket::serde::{Serialize, Serializer};

use crate::backend::database::{Database, Referenceable};

#[derive(Debug, Clone)]
pub struct ForeignKeyStorage<'a> {
    database: &'a Database,
    cache: HashMap<&'static str, Vec<(String, String)>>,
}

impl<'a> From<&'a Database> for ForeignKeyStorage<'a> {
    fn from(value: &'a Database) -> Self {
        Self {
            database: value,
            cache: HashMap::new(),
        }
    }
}

impl<'a> ForeignKeyStorage<'a> {
    pub fn add<T: Referenceable>(&mut self) -> Result<(), crate::backend::database::Error> {
        if self.cache.contains_key(&T::TABLE_NAME) {
            return Ok(());
        }

        self.cache.insert(
            T::TABLE_NAME,
            <T as Referenceable>::generate_descriptions(self.database)?
                .into_iter()
                .map(|value| (value.0.to_string(), value.1))
                .collect(),
        );

        Ok(())
    }
}

impl<'a> Serialize for ForeignKeyStorage<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.cache.serialize(serializer)
    }
}
