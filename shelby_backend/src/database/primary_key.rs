use std::str::FromStr;

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
        PrimaryKey::from_str(v).map_err(|error| E::custom(error.to_string()))
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

impl<T: IndexableDatebaseEntry> std::str::FromStr for PrimaryKey<T> {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Attempt to parse the input string directly as an integer
        if let Ok(id) = s.parse::<i64>() {
            return Ok(PrimaryKey(id, std::marker::PhantomData));
        }

        // Split the string by '/' and try to parse the second part as i64
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidFormat);
        }
        let table_name = parts[1];
        if table_name != T::TABLE_NAME {
            return Err(ParseError::TableNameMismatch);
        }
        let id = parts[2].parse::<i64>().map_err(ParseError::ParseIntError)?;
        Ok(PrimaryKey(id, std::marker::PhantomData))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidFormat,
    TableNameMismatch,
    ParseIntError(std::num::ParseIntError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidFormat => write!(f, "Invalid format"),
            ParseError::TableNameMismatch => write!(f, "Table name mismatch"),
            ParseError::ParseIntError(err) => write!(f, "Parse int error: {}", err),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use crate::{
        database::{PrimaryKey, PrimaryKeyParseError},
        person::Person,
    };

    #[test]
    fn test_parse_valid_string() {
        let parsed = "/persons/123".parse::<PrimaryKey<Person>>();
        assert_eq!(parsed, Ok(PrimaryKey::from(123)));
    }

    #[test]
    fn test_parse_valid_number_string() {
        let parsed = "456".parse::<PrimaryKey<Person>>();
        assert_eq!(parsed, Ok(PrimaryKey::from(456)));
    }

    #[test]
    fn test_parse_invalid_string() {
        let parsed = "/wrong_table/123".parse::<PrimaryKey<Person>>();
        assert_eq!(parsed, Err(PrimaryKeyParseError::TableNameMismatch));
    }

    #[test]
    fn test_deserialize_valid_string() {
        let deserialized: PrimaryKey<Person> = serde_json::from_str(r#""/persons/123""#).unwrap();
        assert_eq!(deserialized, PrimaryKey::from(123));
    }

    #[test]
    fn test_deserialize_valid_number_string() {
        let deserialized: PrimaryKey<Person> = serde_json::from_str(r#""456""#).unwrap();
        assert_eq!(deserialized, PrimaryKey::from(456));
    }

    #[test]
    fn test_deserialize_invalid_string() {
        let deserialized: Result<PrimaryKey<Person>, _> = serde_json::from_str(r#""/persons/123""#);
        assert!(deserialized.is_err());
    }
}
