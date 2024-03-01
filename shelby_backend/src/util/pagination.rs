use crate::database::Selectable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination<T> {
    pub offset: usize,
    pub length: Length,
    pub order: Order,
    column_index: usize,
    selectable: std::marker::PhantomData<*const T>,
}

impl<T: Selectable> Pagination<T> {
    pub fn new(
        column: impl AsRef<str>,
        offset: usize,
        length: Length,
        order: Order,
    ) -> Result<Self, Error> {
        let column = column.as_ref();
        let column_index = T::SORTABLE_COLUMNS
            .iter()
            .position(|&r| r == column)
            .ok_or(Error::InvalidColumn)?;

        Ok(Pagination {
            offset,
            length,
            order,
            column_index,
            selectable: std::marker::PhantomData,
        })
    }

    pub fn end_offset(&self) -> usize {
        self.offset + self.length.0
    }
}

impl<T: Selectable> std::fmt::Display for Pagination<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ORDER BY \"{}\" {} LIMIT {} OFFSET {}",
            T::SORTABLE_COLUMNS[self.column_index],
            self.order,
            self.length,
            self.offset
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Length(usize);

impl Length {
    pub const DEFAULT: Length = Length(5);
    pub const MAXIMUM: Length = Length(100);
}

impl From<usize> for Length {
    fn from(value: usize) -> Self {
        std::cmp::min(Length(value), Self::MAXIMUM)
    }
}

impl From<Length> for usize {
    fn from(value: Length) -> Self {
        value.0
    }
}

impl Default for Length {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<usize> for Length {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidColumn,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The selected column does not exist")
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Ascending,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::User;

    #[test]
    fn test_pagination_new() {
        let pagination: Pagination<User> =
            Pagination::new("id", 1, Length::DEFAULT, Order::Ascending).expect("valid pagination");
        assert_eq!(pagination.offset, 1);
        assert_eq!(pagination.length, Length::DEFAULT);
    }

    #[test]
    fn test_pagination_new_unknown_column() {
        assert_eq!(
            Pagination::<User>::new("unknown_column", 1, Length::DEFAULT, Order::Ascending),
            Err(Error::InvalidColumn)
        );
    }

    #[test]
    fn test_sql() {
        assert_eq!(
            Pagination::<User>::new("id", 1, Length::from(30), Order::Ascending)
                .expect("valid pagination")
                .to_string(),
            String::from("ORDER BY \"id\" ASC LIMIT 30 OFFSET 1")
        );
    }

    #[test]
    fn test_length_max_size() {
        assert_eq!(Length::from(Length::MAXIMUM.0 + 100), Length::MAXIMUM);
    }

    #[test]
    fn test_length_default() {
        assert_eq!(Length::DEFAULT, Length::default());
    }
}
