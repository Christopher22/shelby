use std::{path::Display, str::FromStr};

use crate::backend::{
    accounting::{Account, CostCenter},
    database::{DefaultGenerator, Insertable, PrimaryKey},
    document::Document,
    user::User,
    util::Date,
};

crate::backend::database::make_struct!(
    #[derive(serde::Serialize, serde::Deserialize)]
    #[table("entries")]
    #[dependencies((Document, Account, CostCenter))]
    #[impl_select(true, testing: true)]
    Entry {
        evidence: PrimaryKey<Document>,
        account: PrimaryKey<Account>,
        cost_center: PrimaryKey<CostCenter>,
        amount: Amount,
        description: String
    }
);

impl DefaultGenerator for Entry {
    fn create_default(database: &crate::backend::database::Database) -> Self {
        let evidence = Document::create_default(&database)
            .insert(&database)
            .expect("valid evidence");
        let account = Account::create_default(&database)
            .insert(&database)
            .expect("valid account");
        let cost_center = CostCenter::default()
            .insert(&database)
            .expect("valid cost center");

        Entry {
            evidence,
            account,
            cost_center,
            amount: 32i64.into(),
            description: String::new(),
        }
    }
}

/// A amount of money with two digits after the comma. This type will never have floating point issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(i64);

impl Amount {
    pub fn new(integer_part: i64, fractional_part: i64) -> Result<Self, AmountError> {
        if !(0..100).contains(&fractional_part) {
            return Err(AmountError::FractionTooLarge);
        }
        Ok(Amount(integer_part * 100 + fractional_part))
    }
}

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Amount(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

impl From<i64> for Amount {
    fn from(value: i64) -> Self {
        Amount(value * 100)
    }
}

impl rusqlite::ToSql for Amount {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(self.0.into()))
    }
}

impl rusqlite::types::FromSql for Amount {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).map(Amount)
    }
}

impl<'de> serde::Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AmountVisior)
    }
}

impl serde::Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:0>2}", self.0 / 100, self.0 % 100)
    }
}

impl std::str::FromStr for Amount {
    type Err = AmountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components: Vec<_> = s.split(|c| c == ',' || c == '.').collect();
        if components.len() > 2 {
            return Err(AmountError::DecimalSeparatorIncluded);
        }

        let integer_part = components
            .first()
            .ok_or_else(|| panic!("At least one value should be there"))
            .and_then(|value| i64::from_str(*value))?;

        Ok(match components.get(1) {
            Some(value) => {
                let fractional_part = i64::from_str(*value)?;
                Amount::new(integer_part, fractional_part)?
            }
            None => integer_part.into(),
        })
    }
}

impl crate::backend::database::DatabaseType for Amount {
    const RAW_COLUMN_VALUE: &'static str = "INTEGER";
    const COLUMN_VALUE: &'static str = "INTEGER NOT NULL";
    const IS_SORTABLE: bool = true;
}

/// An possible error occuring during construction of an amount of money.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    FractionTooLarge,
    DecimalSeparatorIncluded,
    InvalidNumber,
}

impl std::fmt::Display for AmountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AmountError::FractionTooLarge => "The fractional component has more than two digits",
            AmountError::DecimalSeparatorIncluded => {
                "There are more than two seperated blocks in the value"
            }
            AmountError::InvalidNumber => "The provided numbers are invalid",
        })
    }
}

impl std::error::Error for AmountError {}

impl From<std::num::ParseIntError> for AmountError {
    fn from(_: std::num::ParseIntError) -> Self {
        Self::InvalidNumber
    }
}
struct AmountVisior;

impl<'de> serde::de::Visitor<'de> for AmountVisior {
    type Value = Amount;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an amount of money like 123.00, 123,00 or 123")
    }

    fn visit_i32<E>(self, amount: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i64(amount as i64)
    }

    fn visit_i64<E>(self, amount: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Amount::from(amount))
    }

    fn visit_u32<E>(self, amount: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i64(amount as i64)
    }

    fn visit_u64<E>(self, amount: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i64(amount as i64)
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Amount::from_str(&format!("{:.2}", v)).map_err(|error| E::custom(error.to_string()))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Amount::from_str(v).map_err(|error| E::custom(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_new_valid() {
        assert_eq!(Amount::new(123, 0), Ok(Amount(12300)));
        assert_eq!(Amount::new(123, 50), Ok(Amount(12350)));
    }

    #[test]
    fn test_amount_new_empty_fraction() {
        assert_eq!(Amount::new(123, 0), Ok(Amount(12300)));
    }

    #[test]
    fn test_amount_new_invalid_fraction() {
        assert_eq!(Amount::new(123, 100), Err(AmountError::FractionTooLarge));
    }

    #[test]
    fn test_amount_display() {
        assert_eq!(Amount::new(123, 45).unwrap().to_string(), "123.45");
        assert_eq!(Amount::from(123).to_string(), "123.00");
    }

    #[test]
    fn test_amount_from_str_valid() {
        assert_eq!("123.45".parse::<Amount>(), Ok(Amount(12345)));
        assert_eq!("123,45".parse::<Amount>(), Ok(Amount(12345)));
        assert_eq!("123".parse::<Amount>(), Ok(Amount(12300)));
    }

    #[test]
    fn test_amount_from_str_invalid() {
        assert_eq!(
            "123.456".parse::<Amount>(),
            Err(AmountError::FractionTooLarge)
        );
        assert_eq!(
            "123,456".parse::<Amount>(),
            Err(AmountError::FractionTooLarge)
        );
        assert_eq!(
            "12.345".parse::<Amount>(),
            Err(AmountError::FractionTooLarge)
        );
        assert_eq!(
            "12,345".parse::<Amount>(),
            Err(AmountError::FractionTooLarge)
        );
        assert_eq!(
            "100.000,345".parse::<Amount>(),
            Err(AmountError::DecimalSeparatorIncluded)
        );
    }

    #[test]
    fn test_amount_add() {
        assert_eq!(Amount(100) + Amount(200), Amount(300));
    }

    #[test]
    fn test_amount_sub() {
        assert_eq!(Amount(300) - Amount(100), Amount(200));
    }

    #[test]
    fn test_amount_serialize() {
        assert_eq!(
            serde_json::to_string(&Amount(12345)).unwrap(),
            r#""123.45""#
        );
    }

    #[test]
    fn test_amount_deserialize_valid() {
        assert_eq!(
            serde_json::from_str::<Amount>(r#""123.45""#).unwrap(),
            Amount(12345)
        );
    }

    #[test]
    fn test_amount_deserialize_valid_integer() {
        assert_eq!(
            serde_json::from_str::<Amount>("123").unwrap(),
            Amount(12300)
        );
        // We round the value
        assert_eq!(
            serde_json::from_str::<Amount>("-123").unwrap(),
            Amount(-12300)
        );
    }

    #[test]
    fn test_amount_deserialize_valid_float() {
        assert_eq!(
            serde_json::from_str::<Amount>(r#"123.45"#).unwrap(),
            Amount(12345)
        );
        // We round the value
        assert_eq!(
            serde_json::from_str::<Amount>(r#"123.454"#).unwrap(),
            Amount(12345)
        );
    }

    #[test]
    fn test_amount_deserialize_invalid() {
        assert!(serde_json::from_str::<Amount>(r#""123.456""#).is_err());
    }
}
