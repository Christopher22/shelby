use serde::Serialize;

use crate::backend::{
    database::{Database, Error, PrimaryKey, Record, SelectableByPrimaryKey},
    person::{Group, Membership as PersonMembership, Person},
};

use super::{util::Map, ForeignKeyStorage};

pub struct GroupOverview<'a> {
    foreign_keys: ForeignKeyStorage<'a, Map>,
    description: String,
    elements: Vec<(PrimaryKey<Person>, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MembershipOverview {
    pub person: String,
    pub membership_path: String,
    pub comment: String,
}

impl<'a> GroupOverview<'a> {
    pub fn load(database: &'a Database, group: Record<Group>) -> Result<Self, Error> {
        let elements: Vec<_> = PersonMembership::find_all_members(database, group.identifier)?
            .into_iter()
            .map(|value| (value.person, value.comment.unwrap_or_default()))
            .collect();
        let description = group.into_inner().description;

        let mut foreign_keys = ForeignKeyStorage::from(database);
        foreign_keys.add::<Person>()?;
        Ok(GroupOverview {
            foreign_keys,
            description,
            elements,
        })
    }
}

impl<'a> super::Renderable for GroupOverview<'a> {
    const TEMPLATE: &'static str = "group";

    fn generate_context(self) -> impl serde::Serialize {
        let rows: Vec<_> = self
            .elements
            .into_iter()
            .map(|(primary_key, comment)| MembershipOverview {
                person: self
                    .foreign_keys
                    .get(primary_key)
                    .unwrap_or_default()
                    .to_owned(),
                comment,
                membership_path: primary_key.to_string(),
            })
            .collect();

        rocket_dyn_templates::context! {
            description: self.description,
            rows: rows,
            persons: ForeignKeyStorage::<'_, crate::frontend::util::List>::from(self.foreign_keys)
        }
    }
}
