use rocket::{
    data::ToByteUnit,
    form::{self, DataField, Errors, FromFormField, ValueField},
};

use crate::backend::database::Selectable;

/// A subsection of selection results.
#[derive(Debug, Copy, PartialEq, Eq)]
pub struct Pagination<T: Selectable> {
    pub offset: usize,
    pub limit: Limit,
    pub order: Order,
    pub column: Column<T>,
}

impl<T: Selectable> Clone for Pagination<T> {
    fn clone(&self) -> Self {
        Self {
            offset: self.offset.clone(),
            limit: self.limit.clone(),
            order: self.order.clone(),
            column: self.column.clone(),
        }
    }
}

impl<T: Selectable> Default for Pagination<T> {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            limit: Default::default(),
            order: Default::default(),
            column: Default::default(),
        }
    }
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

    /// Generate the next pagination element if there appears to be elements left.
    pub fn next(&self, num_recieved_elements: usize) -> Option<Self> {
        if num_recieved_elements < self.limit.0 {
            return None;
        }
        let mut next_pagination = self.clone();
        next_pagination.offset += self.limit.0;
        Some(next_pagination)
    }

    /// Generate the previous pagination element.
    pub fn previous(&self) -> Option<Self> {
        if self.offset == 0 {
            return None;
        }
        let mut last_pagination = self.clone();
        last_pagination.offset = last_pagination
            .offset
            .saturating_sub(last_pagination.limit.0);
        Some(last_pagination)
    }

    pub fn end_offset(&self) -> usize {
        self.offset + self.limit.0
    }

    /// Prepare creating a SQL string.
    pub fn display_sql<'a>(&'a self) -> impl 'a + std::fmt::Display {
        DisplaySql(self)
    }

    /// Prepare creating an URL string.
    pub fn display_url<'a>(&'a self) -> impl 'a + std::fmt::Display {
        DisplayUrl(self)
    }
}

struct DisplaySql<'a, T: Selectable>(&'a Pagination<T>);

impl<'a, T: Selectable> std::fmt::Display for DisplaySql<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ORDER BY {} {} LIMIT {} OFFSET {}",
            self.0.column, self.0.order, self.0.limit, self.0.offset
        )
    }
}

struct DisplayUrl<'a, T: Selectable>(&'a Pagination<T>);

impl<'a, T: Selectable> std::fmt::Display for DisplayUrl<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "?column={}&order={}&limit={}&offset={}",
            self.0.column.as_str(),
            self.0.order,
            self.0.limit,
            self.0.offset
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

#[derive(Debug, Copy, PartialEq, Eq)]
pub struct Column<T: Selectable>(usize, std::marker::PhantomData<fn() -> T>);

impl<T: Selectable> Column<T> {
    /// Return the column name as a string.
    pub fn as_str(&self) -> &'static str {
        T::SORTABLE_COLUMNS[self.0]
    }
}

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
        write!(f, "\"{}\"", self.as_str())
    }
}

impl<T: Selectable> Default for Column<T> {
    fn default() -> Self {
        Self(0, std::marker::PhantomData)
    }
}

impl<T: Selectable> Clone for Column<T> {
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
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
                .display_sql()
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

    #[test]
    fn test_next() {
        let pagination = Pagination::<User> {
            offset: 0,
            limit: Limit::from(3),
            ..Default::default()
        };

        // Lets simulate 5 existings elements in database
        let next_pagination = pagination.next(3).expect("valid pagination");
        assert_eq!(next_pagination.offset, 3);
        assert_eq!(pagination.next(2), None);
    }

    #[test]
    fn test_next_pathologic() {
        let pagination = Pagination::<User> {
            offset: 0,
            limit: Limit::from(3),
            ..Default::default()
        };

        // Lets simulate 5 existings elements in database - however, we received more elements than we would expect given the limit.
        let next_pagination = pagination.next(4).expect("valid pagination");
        assert_eq!(next_pagination.offset, 3);
        assert_eq!(pagination.next(2), None);
    }

    #[test]
    fn test_previous() {
        let pagination = Pagination::<User> {
            offset: 0,
            limit: Limit::from(3),
            ..Default::default()
        };

        let next_pagination = pagination.next(5).expect("valid pagination");
        assert_eq!(next_pagination.previous(), Some(pagination));
    }
}
