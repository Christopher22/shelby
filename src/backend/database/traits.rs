use rusqlite::OptionalExtension;

use super::{Database, Error, PrimaryKey, Record};
use crate::backend::util::Pagination;

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

/// An database indexable through an single primary key.
pub trait Indexable: DatabaseEntry {}

/// An value insertable in the database.
pub trait Insertable: DatabaseEntry + Indexable + DefaultGenerator {
    /// The statement for insert the InsertValue WITHOUT explicit primary key.
    const STATEMENT_INSERT: &'static str;

    /// A (mostly tuple-based) values which could be inserted.
    type InsertValue<'a>: rusqlite::Params
    where
        Self: 'a;

    /// Convert the value as the insert values.
    fn serialize_sql<'a>(&'a self) -> Self::InsertValue<'a>;

    /// Insert the value with a given primary key.
    fn insert(&self, database: &Database) -> Result<PrimaryKey<Self>, Error> {
        Ok(database
            .connection
            .execute(Self::STATEMENT_INSERT, self.serialize_sql())
            .map(|_| PrimaryKey::from(database.connection.last_insert_rowid()))?)
    }

    /// Insert the value with a given primary key. A convenience function to get the record after the insertion.
    fn insert_record(self, database: &Database) -> Result<Record<Self>, Error> {
        Self::insert(&self, database).map(|identifier| Record {
            identifier,
            value: self,
        })
    }
}

pub trait Selectable: DatabaseEntry + Indexable {
    /// The public output. Other than the value itself, this value should be renderable in JSON without leaking sensible information.
    type Output: From<Record<Self>>;

    /// The value which should be extracted from the row.
    type SelectValue<'a>: TryFrom<&'a rusqlite::Row<'a>, Error = rusqlite::Error>;

    /// The statement for selecting all entries.
    const STATEMENT_SELECT_ALL: &'static str;

    /// The sortable values within a table.
    const SORTABLE_COLUMNS: &'static [&'static str];

    /// Deserialize the database value into a Record.
    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> Self::Output;

    /// Select all the elements from the database.
    fn select_all(database: &Database) -> Result<Vec<Self::Output>, Error> {
        let mut stmt = database.connection.prepare(Self::STATEMENT_SELECT_ALL)?;
        let iterator = stmt.query_map((), |row| {
            Self::SelectValue::try_from(row).map(Self::deserialize_sql)
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }

    fn select_all_sorted(
        database: &Database,
        selection: Pagination<Self>,
    ) -> Result<Vec<Self::Output>, Error> {
        let statement = format!("{} {}", Self::STATEMENT_SELECT_ALL, selection);
        let mut stmt = database.connection.prepare(&statement)?;
        let iterator = stmt.query_map((), |row| {
            Self::SelectValue::try_from(row).map(Self::deserialize_sql)
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }
}

pub trait SelectableByPrimaryKey: Selectable + Indexable {
    /// The statement for select WITH explicit primary key.
    const STATEMENT_SELECT: &'static str;

    /// Select an element and parse it.
    fn select(database: &Database, index: PrimaryKey<Self>) -> Result<Self::Output, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index.0,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })?)
    }

    /// Try to select a element which primary key was not validated.
    fn try_select(database: &Database, index: i64) -> Result<Option<Self::Output>, Error> {
        Ok(database
            .connection
            .query_row(Self::STATEMENT_SELECT, (index,), |row| {
                Self::SelectValue::try_from(row).map(Self::deserialize_sql)
            })
            .optional()?)
    }
}

/// An trait for creating a default for objects with complex constraints like foreign keys requiering database access.
pub trait DefaultGenerator {
    /// Create the default element.
    fn create_default(database: &Database) -> Self;
}

impl<T: DatabaseEntry + Default> DefaultGenerator for T {
    fn create_default(_: &Database) -> Self {
        Default::default()
    }
}

/// The information how a value is encoded in a table.
pub trait DatabaseType: rusqlite::types::FromSql + rusqlite::types::ToSql {
    /// The "pure" column value
    const RAW_COLUMN_VALUE: &'static str;

    /// The column value which should normaly be the RAW_COLUMN_VALUE + ' NOT NULL'.
    const COLUMN_VALUE: &'static str;

    /// Indicate if it makes sense to sort an type.
    const IS_SORTABLE: bool;
}

macro_rules! create_database_type {
    ($name: ty => $value: expr; sortable: $is_sortable: expr) => {
        impl crate::backend::database::DatabaseType for $name {
            const RAW_COLUMN_VALUE: &'static str = $value;
            const COLUMN_VALUE: &'static str = const_format::concatcp!($value, " NOT NULL");
            const IS_SORTABLE: bool = $is_sortable;
        }
    };
}

create_database_type!(bool => "BOOL"; sortable: false);
create_database_type!(u32 => "INTEGER"; sortable: true);
create_database_type!(String => "TEXT"; sortable: false);
create_database_type!(crate::backend::Date => "DATETIME"; sortable: true);
create_database_type!(Vec<u8> => "BLOB"; sortable: false);

impl<T: crate::backend::database::Indexable> DatabaseType
    for crate::backend::database::PrimaryKey<T>
{
    const RAW_COLUMN_VALUE: &'static str = "INTEGER";
    const COLUMN_VALUE: &'static str = "INTEGER NOT NULL";
    const IS_SORTABLE: bool = true;
}

impl<T: DatabaseType> DatabaseType for Option<T> {
    const RAW_COLUMN_VALUE: &'static str = T::RAW_COLUMN_VALUE;
    const COLUMN_VALUE: &'static str = T::RAW_COLUMN_VALUE;
    const IS_SORTABLE: bool = false;
}
