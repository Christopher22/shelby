use rocket::serde::Serialize;
use rocket_dyn_templates::context;
use shelby_backend::{Database, IndexableDatebaseEntry, Record};

use super::Renderable;

pub struct TableRenderer<const N: usize, T: RenderableDatabaseEntry<N>>(
    Vec<[String; N]>,
    std::marker::PhantomData<*const T>,
);

impl<const N: usize, T: RenderableDatabaseEntry<N>> Renderable for TableRenderer<N, T>
where
    [&'static str; N]: Serialize,
    [String; N]: Serialize,
{
    const TEMPLATE: &'static str = "table";

    fn generate_context(&self) -> impl Serialize {
        context! {
            headers: &T::COLUMNS,
            rows: &self.0
        }
    }
}

pub trait RenderableDatabaseEntry<const N: usize>: IndexableDatebaseEntry {
    const COLUMNS: [&'static str; N];

    fn generate_table_row(entry: Record<Self>) -> [String; N];

    /// Create a list for rendering all elements.
    fn prepare_rendering_all(
        database: &Database,
    ) -> Result<TableRenderer<N, Self>, shelby_backend::Error> {
        Ok(TableRenderer(
            Self::select_all(database)?
                .into_iter()
                .map(Self::generate_table_row)
                .collect(),
            std::marker::PhantomData,
        ))
    }
}

impl RenderableDatabaseEntry<3> for shelby_backend::person::Person {
    const COLUMNS: [&'static str; 3] = ["Name", "Address", "E-Mail"];

    fn generate_table_row(entry: Record<Self>) -> [String; 3] {
        let value = entry.value;
        [value.name, value.address, value.email.unwrap_or_default()]
    }
}

impl RenderableDatabaseEntry<2> for shelby_backend::person::Group {
    const COLUMNS: [&'static str; 2] = ["Group", "Description"];

    fn generate_table_row(group: Record<Self>) -> [String; 2] {
        [group.identifier.to_string(), group.value.description]
    }
}

impl RenderableDatabaseEntry<4> for shelby_backend::document::Document {
    const COLUMNS: [&'static str; 4] = ["Name", "From", "To", "Description"];

    fn generate_table_row(document: Record<Self>) -> [String; 4] {
        [
            document.identifier.to_string(),
            document.from_person.to_string(),
            document.to_person.to_string(),
            document.value.description.unwrap_or_default(),
        ]
    }
}