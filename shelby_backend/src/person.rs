use crate::database::{Database, DatabaseEntry, Error, PrimaryKey};
use crate::Date;

crate::database::make_struct!(
    Person (Table with derived Default: "persons") depends on () => {
        name: String => "STRING NOT NULL",
        address: String => "STRING NOT NULL",
        email: Option<String> => "STRING",
        birthday: Option<Date> => "DATETIME",
        comment: Option<String> => "STRING"
    }
);

crate::database::make_struct!(
    Group (Table with derived Default: "groups") depends on () => {
        description: String => "STRING NOT NULL"
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

    pub fn insert(&self, database: &Database) -> Result<usize, Error> {
        Ok(database.connection.execute(
            "INSERT INTO memberships (person_id, group_id, updated, comment) VALUES (?, ?, ?, ?)",
            (self.person.0, self.group.0, &self.updated, &self.comment),
        )?)
    }
}

#[cfg(test)]
mod membership_tests {
    use crate::database::{Database, DatabaseEntry, IndexableDatebaseEntry, PrimaryKey};

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
    fn test_no_members() {
        let (database, _, group) = setup_database();
        assert_eq!(Membership::find_all_members(&database, group), Ok(vec![]));
    }

    #[test]
    fn test_no_group() {
        let (database, (p1, _, _), _) = setup_database();
        assert_eq!(Membership::find_all_memberships(&database, p1), Ok(vec![]));
    }

    #[test]
    fn test_membership() {
        let (database, (p1, _, _), g1) = setup_database();

        let membership = Membership {
            person: p1.clone(),
            group: g1.clone(),
            updated: Some(crate::Date::today()),
            comment: Some(String::from("Example")),
        };

        membership.insert(&database).expect("Valid insert");

        assert_eq!(
            Membership::find_all_members(&database, g1.clone()),
            Ok(vec![membership.clone()])
        );
        assert_eq!(
            Membership::find_all_memberships(&database, p1),
            Ok(vec![membership])
        );
    }
}
