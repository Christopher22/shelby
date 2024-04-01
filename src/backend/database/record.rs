use serde::{Deserialize, Serialize};

use super::{Indexable, PrimaryKey};

/// A record with associated, numerical primary key.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<T>
where
    T: Indexable,
{
    pub identifier: PrimaryKey<T>,
    #[serde(flatten)]
    pub value: T,
}

impl<T: Indexable> Record<T> {
    /// Take ownership over the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> std::ops::Deref for Record<T>
where
    T: Indexable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
