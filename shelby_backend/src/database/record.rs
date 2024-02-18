use serde::{Deserialize, Serialize};

use super::{IndexableDatebaseEntry, PrimaryKey};

/// A record with associated, numerical primary key.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<T>
where
    T: IndexableDatebaseEntry,
{
    pub identifier: PrimaryKey<T>,
    #[serde(flatten)]
    pub value: T,
}

impl<T> std::ops::Deref for Record<T>
where
    T: IndexableDatebaseEntry,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
