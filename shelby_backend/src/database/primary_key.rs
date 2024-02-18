use super::IndexableDatebaseEntry;

/// The primary key of a record.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryKey<T: IndexableDatebaseEntry>(
    pub(crate) i64,
    std::marker::PhantomData<*const T>,
);

unsafe impl<T: IndexableDatebaseEntry> std::marker::Send for PrimaryKey<T> {}
unsafe impl<T: IndexableDatebaseEntry> std::marker::Sync for PrimaryKey<T> {}

#[derive(Default)]
struct PrimaryKeyVisitor<T: IndexableDatebaseEntry>(std::marker::PhantomData<*const T>);

impl<'de, T: IndexableDatebaseEntry> serde::de::Visitor<'de> for PrimaryKeyVisitor<T> {
    type Value = PrimaryKey<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a path in the form of '/{}/(<number>' or a u64",
            T::TABLE_NAME
        )
    }

    fn visit_i64<E>(self, primary_key: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(PrimaryKey::from(primary_key))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut iterator = v.split('/').rev();

        let primary_key: i64 =
            iterator
                .next()
                .and_then(|value| value.parse().ok())
                .ok_or(E::invalid_value(
                    serde::de::Unexpected::Other("invalid number"),
                    &self,
                ))?;

        match iterator.next() {
            Some(value) if value == T::TABLE_NAME => Ok(PrimaryKey::from(primary_key)),
            Some(_) => Err(E::invalid_value(
                serde::de::Unexpected::Other("invalid identifier"),
                &self,
            )),
            None => Err(E::invalid_value(
                serde::de::Unexpected::Other("invalid path"),
                &self,
            )),
        }
    }
}

impl<'de, T: IndexableDatebaseEntry> serde::Deserialize<'de> for PrimaryKey<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(PrimaryKeyVisitor::<T>(std::marker::PhantomData))
    }
}

impl<T: IndexableDatebaseEntry> serde::Serialize for PrimaryKey<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<T: IndexableDatebaseEntry> std::fmt::Display for PrimaryKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}/{}", T::TABLE_NAME, self.0)
    }
}

impl<T: IndexableDatebaseEntry> Clone for PrimaryKey<T> {
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
    }
}

impl<T: IndexableDatebaseEntry> Copy for PrimaryKey<T> {}

impl<T: IndexableDatebaseEntry> From<i64> for PrimaryKey<T> {
    fn from(value: i64) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

impl<T: IndexableDatebaseEntry> rusqlite::ToSql for PrimaryKey<T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(self.0.into()))
    }
}

impl<T: IndexableDatebaseEntry> rusqlite::types::FromSql for PrimaryKey<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).map(PrimaryKey::from)
    }
}
