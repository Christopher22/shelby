use std::collections::HashMap;

use rocket::serde::{Serialize, Serializer};

use crate::backend::database::{Database, Indexable, PrimaryKey, Referenceable};

/// The (non-generic) data structure used within the foreign key cache.
pub trait Container: Serialize {
    fn from<T: Indexable>(raw_container: Vec<(PrimaryKey<T>, String)>) -> Self;
}

/// A ordered list of key and human-readable style.
pub struct List(Vec<(String, String)>);

impl Container for List {
    fn from<T: Indexable>(raw_container: Vec<(PrimaryKey<T>, String)>) -> Self {
        Self(
            raw_container
                .into_iter()
                .map(|value| (value.0.to_string(), value.1))
                .collect(),
        )
    }
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// A fast map which could be used for queries.
pub struct Map(HashMap<i64, String>);

impl Container for Map {
    fn from<T: Indexable>(raw_container: Vec<(PrimaryKey<T>, String)>) -> Self {
        Self(
            raw_container
                .into_iter()
                .map(|value| (value.0 .0, value.1))
                .collect(),
        )
    }
}

impl Serialize for Map {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// A container for storing human-readable representations for foreign keys without generics.
#[derive(Debug, Clone)]
pub struct ForeignKeyStorage<'a, C: Container = List> {
    database: &'a Database,
    cache: HashMap<&'static str, C>,
}

impl<'a, C: Container> From<&'a Database> for ForeignKeyStorage<'a, C> {
    fn from(value: &'a Database) -> Self {
        Self {
            database: value,
            cache: HashMap::new(),
        }
    }
}

impl<'a, C: Container> ForeignKeyStorage<'a, C> {
    /// Load a foreign key into the cache.
    pub fn add<T: Referenceable>(&mut self) -> Result<(), crate::backend::database::Error> {
        if self.cache.contains_key(&T::TABLE_NAME) {
            return Ok(());
        }

        self.cache.insert(
            T::TABLE_NAME,
            C::from(<T as Referenceable>::generate_descriptions(self.database)?),
        );

        Ok(())
    }
}

impl<'a> ForeignKeyStorage<'a, Map> {
    /// Get the corresponding representation for a primary key.
    pub fn get<T: Referenceable>(&self, primary_key: PrimaryKey<T>) -> Option<&str> {
        self.cache
            .get(&T::TABLE_NAME)
            .and_then(|value| value.0.get(&primary_key.0))
            .map(|x| x.as_str())
    }
}

impl<'a, C: Container> Serialize for ForeignKeyStorage<'a, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.cache.serialize(serializer)
    }
}

impl<'a> From<ForeignKeyStorage<'a, Map>> for ForeignKeyStorage<'a, List> {
    fn from(value: ForeignKeyStorage<'a, Map>) -> Self {
        Self {
            database: value.database,
            cache: value
                .cache
                .into_iter()
                .map(|table| {
                    (
                        table.0,
                        List(
                            table
                                .1
                                 .0
                                .into_iter()
                                .map(|foreign_key| {
                                    (format!("/{}/{}", table.0, foreign_key.0), foreign_key.1)
                                })
                                .collect(),
                        ),
                    )
                })
                .collect(),
        }
    }
}
