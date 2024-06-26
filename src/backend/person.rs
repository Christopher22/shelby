use crate::backend::{
    database::{Database, DatabaseEntry, Error, PrimaryKey},
    Date,
};

crate::backend::database::make_struct!(
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[table("persons")]
    #[dependencies(())]
    #[impl_select(true, testing: true, description: "name")]
    Person {
        name: String,
        address: String,
        email: Option<String>,
        birthday: Option<Date>,
        comment: Option<String>
    }
);

crate::backend::database::make_struct!(
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[table("groups")]
    #[dependencies(())]
    #[impl_select(true, testing: true, description: "description")]
    Group {
        description: String
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    pub person: PrimaryKey<Person>,
    pub group: PrimaryKey<Group>,
    pub updated: Option<Date>,
    pub comment: Option<String>,
}

impl DatabaseEntry for Membership {
    type DependsOn = (Person, Group);

    const TABLE_NAME: &'static str = "memberships";
    const STATEMENT_CREATE_TABLE: &'static str = std::concat!(
        "CREATE TABLE IF NOT EXISTS memberships (
            person_id INTEGER NOT NULL, group_id INTEGER NOT NULL, updated DATETIME, comment STRING,
            PRIMARY KEY (person_id, group_id),
            FOREIGN KEY (person_id) REFERENCES persons(id), 
            FOREIGN KEY (group_id) REFERENCES groups(id)
        )"
    );
}

impl Membership {
    /// Find all meberships of a single person.
    pub fn find_all_memberships(
        database: &Database,
        person: PrimaryKey<Person>,
    ) -> Result<Vec<Membership>, Error> {
        let mut stmt = database
            .connection
            .prepare("SELECT group_id, updated, comment FROM memberships WHERE person_id = ?")?;

        let iterator = stmt.query_map((person.0,), |row| {
            Ok(Membership {
                person: person.clone(),
                group: PrimaryKey::from(row.get::<usize, i64>(0)?),
                updated: row.get(1)?,
                comment: row.get(2)?,
            })
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }

    /// Find all members of a group.
    pub fn find_all_members(
        database: &Database,
        group: PrimaryKey<Group>,
    ) -> Result<Vec<Membership>, Error> {
        let mut stmt = database
            .connection
            .prepare("SELECT person_id, updated, comment FROM memberships WHERE group_id = ?")?;

        let iterator = stmt.query_map((group.0,), |row| {
            Ok(Membership {
                person: PrimaryKey::from(row.get::<usize, i64>(0)?),
                group: group.clone(),
                updated: row.get(1)?,
                comment: row.get(2)?,
            })
        })?;

        Ok(iterator.filter_map(|value| value.ok()).collect())
    }

    /// Insert a membership into the database.
    pub fn insert(&self, database: &Database) -> Result<usize, Error> {
        Ok(database.connection.execute(
            "INSERT INTO memberships (person_id, group_id, updated, comment) VALUES (?, ?, ?, ?)",
            (self.person.0, self.group.0, &self.updated, &self.comment),
        )?)
    }

    /// Remove a membership from the database.
    pub fn remove(
        person: PrimaryKey<Person>,
        group: PrimaryKey<Group>,
        database: &Database,
    ) -> Result<usize, Error> {
        Ok(database.connection.execute(
            "DELETE FROM memberships WHERE person_id = ? AND group_id = ?",
            (person.0, group.0),
        )?)
    }
}

#[cfg(test)]
mod membership_tests {
    use crate::backend::database::{Database, DatabaseEntry, Insertable, PrimaryKey};

    use super::{Group, Membership, Person};

    fn setup_database() -> (
        Database,
        (PrimaryKey<Person>, PrimaryKey<Person>, PrimaryKey<Person>),
        PrimaryKey<Group>,
    ) {
        let database = Database::plain().expect("valid database");
        Membership::create_table(&database).expect("valid table");

        let p1 = Person {
            name: String::from("Max Mustermann"),
            ..Default::default()
        }
        .insert(&database)
        .expect("valid person");

        let p2 = Person {
            name: String::from("Mariane Mustermann"),
            ..Default::default()
        }
        .insert(&database)
        .expect("valid person");

        let p3 = Person {
            name: String::from("Jane Doe"),
            ..Default::default()
        }
        .insert(&database)
        .expect("valid person");

        let g1 = Group {
            description: String::from("Example"),
        }
        .insert(&database)
        .expect("insert sucessfull");

        (database, (p1, p2, p3), g1)
    }

    #[test]
    fn test_create_table() {
        // This will trigger the create statement.
        setup_database();
    }

    #[test]
    fn test_insert_and_find() {
        let (database, (p1, _, _), g1) = setup_database();

        let membership = Membership {
            person: p1,
            group: g1,
            updated: None,
            comment: Some(String::from("Example")),
        };

        membership.insert(&database).expect("Valid insert");

        let memberships_of_group = Membership::find_all_members(&database, g1.clone()).unwrap();
        assert_eq!(memberships_of_group.len(), 1);
        assert_eq!(memberships_of_group[0], membership);

        let memberships_of_person = Membership::find_all_memberships(&database, p1).unwrap();
        assert_eq!(memberships_of_person.len(), 1);
        assert_eq!(memberships_of_person[0], membership);
    }

    #[test]
    fn test_remove() {
        let (database, (p1, _, _), g1) = setup_database();

        let membership = Membership {
            person: p1,
            group: g1,
            updated: None,
            comment: Some(String::from("Example")),
        };

        membership.insert(&database).expect("Valid insert");

        assert_eq!(Membership::remove(p1, g1, &database), Ok(1));

        assert_eq!(
            Membership::find_all_members(&database, g1.clone()),
            Ok(vec![])
        );
        assert_eq!(Membership::find_all_memberships(&database, p1), Ok(vec![]));
    }

    #[test]
    fn test_find_all_members_with_no_members() {
        let (database, _, group) = setup_database();
        assert_eq!(Membership::find_all_members(&database, group), Ok(vec![]));
    }

    #[test]
    fn test_find_all_memberships_with_no_group() {
        let (database, (p1, _, _), _) = setup_database();
        assert_eq!(Membership::find_all_memberships(&database, p1), Ok(vec![]));
    }
}
