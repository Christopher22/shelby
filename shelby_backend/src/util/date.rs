use chrono::{DateTime, NaiveDate, Utc};

/// A date which is today or in the past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date(NaiveDate);

impl Date {
    /// Get the current date.
    pub fn today() -> Date {
        Date(Utc::now().date_naive())
    }
}

impl ToString for Date {
    fn to_string(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}

impl Default for Date {
    fn default() -> Self {
        Self::today()
    }
}

impl TryFrom<NaiveDate> for Date {
    type Error = Error;

    fn try_from(value: NaiveDate) -> Result<Self, Self::Error> {
        match value <= Utc::now().date_naive() {
            true => Ok(Date(value)),
            false => Err(Error),
        }
    }
}

impl TryFrom<DateTime<Utc>> for Date {
    type Error = Error;

    fn try_from(value: DateTime<Utc>) -> Result<Self, Self::Error> {
        Self::try_from(value.date_naive())
    }
}

impl<'de> serde::Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DateVisitor)
    }
}

impl serde::Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl rusqlite::ToSql for Date {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl rusqlite::types::FromSql for Date {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        // Should we check the database, too?
        NaiveDate::column_result(value).map(Date)
    }
}

#[derive(Default)]
struct DateVisitor;

impl<'de> serde::de::Visitor<'de> for DateVisitor {
    type Value = Date;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a unix timestamp or a string in the form '2000-01-31'"
        )
    }

    fn visit_i64<E>(self, unix_time: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        DateTime::<Utc>::from_timestamp(unix_time, 0)
            .ok_or(Error)
            .and_then(Date::try_from)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Signed(unix_time), &self))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let date = NaiveDate::parse_from_str(v, "%Y-%m-%d")
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))?;
        Date::try_from(date).map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Error;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid timestamp which is in the future")
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Default, Deserialize, PartialEq, Eq)]
    struct OptionalDate {
        optional_date: Option<Date>,
    }

    #[test]
    fn test_date_today() {
        let today = Date::today();
        let now = Utc::now().date_naive();
        assert_eq!(today.0, now);
    }

    #[test]
    fn test_date_default() {
        let default_date = Date::default();
        let now = Utc::now().date_naive();
        assert_eq!(default_date.0, now);
    }

    #[test]
    fn test_date_try_from_naive_date() {
        let naive_date = NaiveDate::from_ymd_opt(2022, 1, 1).expect("valid date");
        let date = Date::try_from(naive_date).unwrap();
        assert_eq!(date.0, naive_date);
    }

    #[test]
    fn test_date_try_from_datetime_utc() {
        let datetime_utc = Utc::now();
        let date = Date::try_from(datetime_utc).unwrap();
        assert_eq!(date.0, datetime_utc.date_naive());
    }

    #[test]
    fn test_date_serialize_to_string() {
        let date = Date(NaiveDate::from_ymd_opt(2022, 2, 18).expect("valid date"));
        let serialized = serde_json::to_string(&date).unwrap();
        assert_eq!(serialized, "\"2022-02-18\"");
    }

    #[test]
    fn test_date_deserialize_from_string() {
        let serialized = "\"2022-02-18\"";
        let deserialized: Date = serde_json::from_str(serialized).unwrap();
        assert_eq!(
            deserialized.0,
            NaiveDate::from_ymd_opt(2022, 2, 18).expect("valid date")
        );
    }

    #[test]
    fn test_date_not_in_future() {
        // Would be interesting, if the software is still used
        let future_date = NaiveDate::from_ymd_opt(3000, 2, 18).expect("valid date");
        assert_eq!(Date::try_from(future_date), Err(Error));
    }

    #[test]
    fn test_date_deserialize_from_string_invalid() {
        let serialized = "\"3000-02-18\"";
        assert!(serde_json::from_str::<Date>(serialized).is_err());
    }

    #[test]
    fn test_optional_missing() {
        let serialized = "{}";
        let deserialized: OptionalDate = serde_json::from_str(serialized).unwrap();
        assert_eq!(deserialized.optional_date, None)
    }
}
