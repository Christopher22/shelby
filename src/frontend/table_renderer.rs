use crate::backend::{
    database::{Database, Record, Selectable},
    document::Document,
    person::{Group, Person},
    user::User,
};
use rocket::serde::Serialize;
use rocket_dyn_templates::context;

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
            title: &T::TITLE,
            headers: &T::COLUMNS,
            url_add: &T::URL_ADD,
            rows: &self.0
        }
    }
}

pub trait RenderableDatabaseEntry<const N: usize>: Selectable {
    /// The title of the entry
    const TITLE: &'static str;

    /// The columns of the table.
    const COLUMNS: [&'static str; N];

    /// The path to the form to create a new element.
    const URL_ADD: &'static str;

    /// Translate a record into a row of strings.
    fn generate_table_row(entry: Self::Output) -> [String; N];

    /// Create a list for rendering all elements.
    fn prepare_rendering_all(
        database: &Database,
    ) -> Result<TableRenderer<N, Self>, crate::backend::database::Error> {
        Ok(TableRenderer(
            Self::select_all(database)?
                .into_iter()
                .map(Self::generate_table_row)
                .collect(),
            std::marker::PhantomData,
        ))
    }
}

impl RenderableDatabaseEntry<3> for Person {
    const TITLE: &'static str = "Contacts";
    const COLUMNS: [&'static str; 3] = ["Name", "Address", "E-Mail"];
    const URL_ADD: &'static str = "/persons/new";

    fn generate_table_row(entry: Record<Self>) -> [String; 3] {
        let value = entry.value;
        [value.name, value.address, value.email.unwrap_or_default()]
    }
}

impl RenderableDatabaseEntry<2> for Group {
    const TITLE: &'static str = "Groups";
    const COLUMNS: [&'static str; 2] = ["Group", "Description"];
    const URL_ADD: &'static str = "/groups/new";

    fn generate_table_row(group: Record<Self>) -> [String; 2] {
        [group.identifier.to_string(), group.value.description]
    }
}

impl RenderableDatabaseEntry<4> for Document {
    const TITLE: &'static str = "Documents";
    const COLUMNS: [&'static str; 4] = ["Name", "From", "To", "Description"];
    const URL_ADD: &'static str = "/documents/new";

    fn generate_table_row(document: <Document as Selectable>::Output) -> [String; 4] {
        [
            document.identifier.to_string(),
            document.from_person.to_string(),
            document.to_person.to_string(),
            document.description.unwrap_or_default(),
        ]
    }
}

impl RenderableDatabaseEntry<4> for User {
    const TITLE: &'static str = "Users";
    const COLUMNS: [&'static str; 4] = ["Identifier", "Name", "Creation date", "Used by"];
    const URL_ADD: &'static str = "/users/new";

    fn generate_table_row(user: <User as Selectable>::Output) -> [String; 4] {
        [
            user.identifier.to_string(),
            user.username.to_string(),
            user.creation_date.to_string(),
            user.related_to
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ]
    }
}
