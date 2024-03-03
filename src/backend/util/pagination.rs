use rocket::{
    data::ToByteUnit,
    form::{self, DataField, Errors, FromFormField, ValueField},
};

use crate::backend::database::Selectable;

/// A subsection of selection results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromForm, Default)]
pub struct Pagination<T: Selectable> {
    #[field(default = 0)]
    pub offset: usize,
    #[field(default = <Limit as Default>::default())]
    pub limit: Limit,
    #[field(default = <Order as Default>::default())]
    pub order: Order,
    #[field(default = <Column<T> as Default>::default())]
    pub column: Column<T>,
}

impl<T: Selectable> Pagination<T> {
    pub fn new(
        column: impl AsRef<str>,
        offset: usize,
        limit: Limit,
        order: Order,
    ) -> Result<Self, Error> {
        let column = Column::try_from(column.as_ref())?;
        Ok(Pagination {
            offset,
            limit,
            order,
            column,
        })
    }

    pub fn end_offset(&self) -> usize {
        self.offset + self.limit.0
    }
}

impl<T: Selectable> std::fmt::Display for Pagination<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ORDER BY {} {} LIMIT {} OFFSET {}",
            self.column, self.order, self.limit, self.offset
        )
    }
}

/// The number of samples within the pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Limit(usize);

impl Limit {
    pub const DEFAULT: Limit = Limit(10);
    pub const MAXIMUM: Limit = Limit(100);
}

impl From<usize> for Limit {
    fn from(value: usize) -> Self {
        std::cmp::min(Limit(value), Self::MAXIMUM)
    }
}

impl From<Limit> for usize {
    fn from(value: Limit) -> Self {
        value.0
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<usize> for Limit {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Limit {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        usize::from_value(field).map(Limit::from)
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        usize::from_data(field).await.map(Limit::from)
    }
}

/// An error retuned during pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The specified column does not exist.
    InvalidColumn,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The selected column does not exist")
    }
}

impl std::error::Error for Error {}

/// The order the values are sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// The items are sorted in ascending order.
    Ascending,
    /// The items are sorted in descending order.
    Descending,
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Order::Ascending => "ASC",
            Order::Descending => "DESC",
        })
    }
}

impl Default for Order {
    fn default() -> Self {
        Self::Descending
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for Order {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        match field.value {
            value if value.eq_ignore_ascii_case("asc") => Ok(Order::Ascending),
            value if value.eq_ignore_ascii_case("desc") => Ok(Order::Descending),
            _ => Err(Errors::new().with_name(field.name)),
        }
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        String::from_data(field)
            .await
            .and_then(|value| match value {
                value if value.eq_ignore_ascii_case("asc") => Ok(Order::Ascending),
                value if value.eq_ignore_ascii_case("desc") => Ok(Order::Descending),
                _ => Err(Errors::new()),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column<T: Selectable>(usize, std::marker::PhantomData<fn() -> T>);

impl<'a, T: Selectable> TryFrom<&'a str> for Column<T> {
    type Error = Error;

    fn try_from(column: &str) -> Result<Self, Self::Error> {
        T::SORTABLE_COLUMNS
            .iter()
            .position(|&r| r == column)
            .ok_or(Error::InvalidColumn)
            .map(|value| Column(value, std::marker::PhantomData))
    }
}

impl<T: Selectable> std::fmt::Display for Column<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", T::SORTABLE_COLUMNS[self.0])
    }
}

impl<T: Selectable> Default for Column<T> {
    fn default() -> Self {
        Self(0, std::marker::PhantomData)
    }
}

#[rocket::async_trait]
impl<'r, T: Selectable> FromFormField<'r> for Column<T> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        String::from_value(field)
            .and_then(|value| Column::try_from(value.as_str()).map_err(|_| Errors::new()))
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        String::from_data(field)
            .await
            .and_then(|value| Column::try_from(value.as_str()).map_err(|_| Errors::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::user::User;

    #[test]
    fn test_pagination_new() {
        let pagination: Pagination<User> =
            Pagination::new("id", 1, Limit::DEFAULT, Order::Ascending).expect("valid pagination");
        assert_eq!(pagination.offset, 1);
        assert_eq!(pagination.limit, Limit::DEFAULT);
    }

    #[test]
    fn test_pagination_new_unknown_column() {
        assert_eq!(
            Pagination::<User>::new("unknown_column", 1, Limit::DEFAULT, Order::Ascending),
            Err(Error::InvalidColumn)
        );
    }

    #[test]
    fn test_sql() {
        assert_eq!(
            Pagination::<User>::new("id", 1, Limit::from(30), Order::Ascending)
                .expect("valid pagination")
                .to_string(),
            String::from("ORDER BY \"id\" ASC LIMIT 30 OFFSET 1")
        );
    }

    #[test]
    fn test_length_max_size() {
        assert_eq!(Limit::from(Limit::MAXIMUM.0 + 100), Limit::MAXIMUM);
    }

    #[test]
    fn test_length_default() {
        assert_eq!(Limit::DEFAULT, <Limit as Default>::default());
    }
}
