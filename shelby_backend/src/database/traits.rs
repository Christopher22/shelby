use rusqlite::OptionalExtension;

use super::{Database, Error, PrimaryKey, Record};

pub trait Dependency {
    fn create_dependencies(database: &Database) -> Result<(), Error>;
}

impl Dependency for () {
    fn create_dependencies(_: &Database) -> Result<(), Error> {
        Ok(())
    }
}

impl<T> Dependency for T
where
    T: DatabaseEntry,
{
    fn create_dependencies(database: &Database) -> Result<(), Error> {
        T::DependsOn::create_dependencies(database)?;
        Ok(database
            .connection
            .execute(T::STATEMENT_CREATE_TABLE, ())
            .map(|_| ())?)
    }
}

impl<T1, T2> Dependency for (T1, T2)
where
    T1: Dependency,
    T2: Dependency,
{
    fn create_dependencies(database: &Database) -> Result<(), Error> {
        T1::create_dependencies(database)?;
        Ok(T2::create_dependencies(database)?)
    }
}

/// An element serialialized in the Database.
pub trait DatabaseEntry: Sized {
    type DependsOn: Dependency;

    const TABLE_NAME: &'static str;
    const STATEMENT_CREATE_TABLE: &'static str;

    /// Create the required table and all dependencies.
    fn create_table(database: &Database) -> Result<(), Error> {
        Ok(Self::create_dependencies(database)?)
    }
}

/// An value insertable in the database.
pub trait IndexableDatebaseEntry: DefaultGenerator {
    /// The statement for select WITH explicit primary key.
    const STATEMENT_SELECT: &'static str;

    /// The statement for selecting all entries.
    const STATEMENT_SELECT_ALL: &'static str;

    /// The statement for insert the InsertValue WITHOUT explicit primary key.
    const STATEMENT_INSERT: &'static str;

    /// A (mostly tuple-based) values which could be inserted.
    type InsertValue<'a>: rusqlite::Params
    where
        Self: 'a;

    /// The value which should be extracted from the row.
    type SelectValue<'a>: TryFrom<&'a rusqlite::Row<'a>, Error = rusqlite::Error>;

    /// Convert the value as the insert values.
    fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a>;

    /// Deserialize the database value into a Record.
    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> Record<Self>;

    /// Select an element and parse it.
    fn select(database: &Database, index: PrimaryKey<Self>) -> Result<Record<Self>, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })?)
    }

    /// Try to select a element which primary key was not validated.
    fn try_select(database: &Database, index: i64) -> Result<Option<Record<Self>>, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })
            .optional()?)
    }

    /// Select all the elements from the database.
    fn select_all(database: &Database) -> Result<Vec<Record<Self>>, Error> {
        let mut stmt = database.connection.prepare(Self::STATEMENT_SELECT_ALL)?;
        let iterator = stmt.query_map((), |row| {
            Self::SelectValue::try_from(row).map(Self::deserialize_sql)
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }

    /// Insert the value with a given primary key.
    fn insert(&self, database: &Database) -> Result<PrimaryKey<Self>, Error> {
        Ok(database
            .connection
            .execute(Self::STATEMENT_INSERT, self.serialize_sql())
            .map(|_| PrimaryKey::from(database.connection.last_insert_rowid()))?)
    }
}

/// An trait for creating a default for objects with complex constraints like foreign keys requiering database access.
pub trait DefaultGenerator: DatabaseEntry {
    /// Create the default element.
    fn create_default(database: &Database) -> Self;
}

impl<T: DatabaseEntry + Default> DefaultGenerator for T {
    fn create_default(_: &Database) -> Self {
        Default::default()
    }
}
