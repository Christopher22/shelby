use crate::backend::{
    accounting::Category,
    database::{Database, Record, Selectable},
    document::Document,
    person::{Group, Person},
    user::User,
    Pagination,
};
use rocket::serde::Serialize;
use rocket_dyn_templates::context;

use super::Renderable;

type ForeignKeyStorage<'a> = super::util::ForeignKeyStorage<'a, super::util::Map>;

pub struct TableRenderer<const N: usize, T: RenderableDatabaseEntry<N>>(
    Vec<[String; N]>,
    Pagination<T>,
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
            rows: &self.0,
            next_url: self.1.next(self.0.len()).map(|value| format!("{}{}", T::url(), value.display_url())),
            previous_url: self.1.previous().map(|value| format!("{}{}", T::url(), value.display_url())),
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

    /// Load required foreign keys before generating the rows.
    fn load_required_foreign_keys(
        _: &mut ForeignKeyStorage<'_>,
    ) -> Result<(), crate::backend::database::Error> {
        Ok(())
    }

    /// Translate a record into a row of strings.
    fn generate_table_row(entry: Self::Output, foreign_keys: &ForeignKeyStorage<'_>)
        -> [String; N];

    /// Create a list for rendering all elements.
    fn prepare_rendering_all(
        database: &Database,
        pagination: Pagination<Self>,
    ) -> Result<TableRenderer<N, Self>, crate::backend::database::Error> {
        let mut foreign_keys = ForeignKeyStorage::from(database);

        Self::load_required_foreign_keys(&mut foreign_keys)?;
        Ok(TableRenderer(
            Self::select_all_sorted(database, pagination.clone())?
                .into_iter()
                .map(|value| Self::generate_table_row(value, &foreign_keys))
                .collect(),
            pagination,
        ))
    }

    /// Extract the URL of this form.
    /// By default, this assumes an URL_ADD ending with "/new."
    fn url() -> &'static str {
        match Self::URL_ADD.find("/new") {
            Some(index) => &Self::URL_ADD[..index],
            None => "",
        }
    }
}

impl RenderableDatabaseEntry<3> for Person {
    const TITLE: &'static str = "Contacts";
    const COLUMNS: [&'static str; 3] = ["Name", "Address", "E-Mail"];
    const URL_ADD: &'static str = "/persons/new";

    fn generate_table_row(entry: Record<Self>, _: &ForeignKeyStorage<'_>) -> [String; 3] {
        let value = entry.value;
        [value.name, value.address, value.email.unwrap_or_default()]
    }
}

impl RenderableDatabaseEntry<1> for Group {
    const TITLE: &'static str = "Groups";
    const COLUMNS: [&'static str; 1] = ["Description"];
    const URL_ADD: &'static str = "/groups/new";

    fn generate_table_row(group: Record<Self>, _: &ForeignKeyStorage<'_>) -> [String; 1] {
        [group.value.description]
    }
}

impl RenderableDatabaseEntry<6> for Document {
    const TITLE: &'static str = "Documents";
    const COLUMNS: [&'static str; 6] =
        ["File", "Recieved", "Processed", "From", "To", "Description"];
    const URL_ADD: &'static str = "/documents/new";

    fn load_required_foreign_keys(
        foreign_key_storage: &mut ForeignKeyStorage<'_>,
    ) -> Result<(), crate::backend::database::Error> {
        foreign_key_storage.add::<Person>()
    }

    fn generate_table_row(
        document: <Document as Selectable>::Output,
        foreign_keys: &ForeignKeyStorage<'_>,
    ) -> [String; 6] {
        [
            format!("<a href=\"{}/pdf\">PDF</a>", document.identifier),
            document.recieved.to_string(),
            document.processed.to_string(),
            foreign_keys
                .get(document.from_person)
                .map(String::from)
                .unwrap_or_else(|| document.from_person.to_string()),
            foreign_keys
                .get(document.to_person)
                .map(String::from)
                .unwrap_or_else(|| document.from_person.to_string()),
            document.description.unwrap_or_default(),
        ]
    }
}

impl RenderableDatabaseEntry<3> for User {
    const TITLE: &'static str = "Users";
    const COLUMNS: [&'static str; 3] = ["Name", "Creation date", "Used by"];
    const URL_ADD: &'static str = "/users/new";

    fn load_required_foreign_keys(
        foreign_key_storage: &mut ForeignKeyStorage<'_>,
    ) -> Result<(), crate::backend::database::Error> {
        foreign_key_storage.add::<Person>()
    }

    fn generate_table_row(
        user: <User as Selectable>::Output,
        foreign_keys: &ForeignKeyStorage<'_>,
    ) -> [String; 3] {
        [
            user.username.to_string(),
            user.creation_date.to_string(),
            user.related_to
                .and_then(|value| foreign_keys.get(value).map(String::from))
                .unwrap_or_default(),
        ]
    }
}

impl RenderableDatabaseEntry<1> for crate::backend::accounting::Category {
    const TITLE: &'static str = "Categories";
    const COLUMNS: [&'static str; 1] = ["Description"];
    const URL_ADD: &'static str = "/categories/new";

    fn generate_table_row(category: Record<Self>, _: &ForeignKeyStorage<'_>) -> [String; 1] {
        [category.value.description]
    }
}

impl RenderableDatabaseEntry<1> for crate::backend::accounting::CostCenter {
    const TITLE: &'static str = "Cost centers";
    const COLUMNS: [&'static str; 1] = ["Description"];
    const URL_ADD: &'static str = "/cost_centers/new";

    fn generate_table_row(cost_center: Record<Self>, _: &ForeignKeyStorage<'_>) -> [String; 1] {
        [cost_center.value.description]
    }
}

impl RenderableDatabaseEntry<3> for crate::backend::accounting::Account {
    const TITLE: &'static str = "Accounts";
    const COLUMNS: [&'static str; 3] = ["Code", "Category", "Description"];
    const URL_ADD: &'static str = "/accounts/new";

    fn load_required_foreign_keys(
        foreign_key_storage: &mut ForeignKeyStorage<'_>,
    ) -> Result<(), crate::backend::database::Error> {
        foreign_key_storage.add::<Category>()
    }

    fn generate_table_row(
        account: Record<Self>,
        foreign_keys: &ForeignKeyStorage<'_>,
    ) -> [String; 3] {
        [
            account.value.code.to_string(),
            foreign_keys
                .get(account.value.category)
                .map(String::from)
                .unwrap_or_else(|| account.category.to_string()),
            account.value.description,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        // Check the URL for a single example.
        assert_eq!(User::url(), "/users");
    }
}
