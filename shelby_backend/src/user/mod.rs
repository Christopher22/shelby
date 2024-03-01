use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::database::{Database, DatabaseEntry, DefaultGenerator, PrimaryKey, Record};
use crate::person::Person;
use crate::Date;

mod password_hash;
pub use self::password_hash::PasswordHash;

crate::database::make_struct!(
    #[derive(Serialize)]
    #[table("users")]
    #[dependencies(Person)]
    #[impl_select(false, testing: true)]
    User {
        username: String,
        password_hash: PasswordHash,
        active: bool,
        creation_date: Date,
        related_to: Option<PrimaryKey<Person>>
    } ("FOREIGN KEY(related_to) REFERENCES persons(id)")
);

impl User {
    /// Select a user by its name.
    pub fn select_by_name(
        database: &Database,
        name: impl AsRef<str>,
    ) -> Result<Option<Record<Self>>, crate::database::Error> {
        const SELECT_BY_NAME_QUERY: &'static str =
            const_format::formatcp!("SELECT * FROM {} WHERE username = ?", User::TABLE_NAME);

        Ok(database
            .connection
            .query_row(SELECT_BY_NAME_QUERY, (name.as_ref(),), |row| {
                <(
                    PrimaryKey<User>,
                    String,
                    PasswordHash,
                    bool,
                    Date,
                    Option<PrimaryKey<Person>>,
                )>::try_from(row)
                .map(|value| Record {
                    identifier: value.0,
                    value: User {
                        username: value.1,
                        password_hash: value.2,
                        active: value.3,
                        creation_date: value.4,
                        related_to: value.5,
                    },
                })
            })
            .optional()?)
    }
}

impl DefaultGenerator for User {
    fn create_default(_: &Database) -> Self {
        User {
            username: String::from("Chris"),
            password_hash: PasswordHash::new("Chris", "test1234"),
            active: true,
            creation_date: Date::today(),
            related_to: None,
        }
    }
}

impl crate::database::Selectable for User {
    /// The public output. Other than the value itself, this value should be renderable in JSON without leaking sensible information.
    type Output = Metadata;

    /// The value which should be extracted from the row.
    type SelectValue<'a> = (
        PrimaryKey<User>,
        String,
        bool,
        Date,
        Option<PrimaryKey<Person>>,
    );

    /// The statement for selecting all entries.
    const STATEMENT_SELECT_ALL: &'static str = const_format::formatcp!(
        "SELECT id, username, active, creation_date, related_to FROM {}",
        User::TABLE_NAME
    );

    const SORTABLE_COLUMNS: &'static [&'static str] = &["id", "creation_date"];

    /// Deserialize the database value into a Record.
    fn deserialize_sql<'a>(value: Self::SelectValue<'a>) -> Self::Output {
        Metadata {
            identifier: value.0,
            username: value.1,
            active: value.2,
            creation_date: value.3,
            related_to: value.4,
        }
    }
}

impl crate::database::SelectableByPrimaryKey for User {
    const STATEMENT_SELECT: &'static str = const_format::concatcp!(
        <User as crate::database::Selectable>::STATEMENT_SELECT_ALL,
        " WHERE id = ?"
    );
}

/// The serialization and serialization of this class is special. The password hash can be deserialized, derived from a raw password, or even skipped and than replaced by an invalid password hash.
impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct UserHelper {
            username: String,
            password_hash: Option<PasswordHash>,
            password: Option<String>,
            active: bool,
            creation_date: Date,
            related_to: Option<PrimaryKey<Person>>,
        }

        let helper = UserHelper::deserialize(deserializer)?;

        let password_hash = match (&helper.password, &helper.password_hash) {
            (Some(value), None) => PasswordHash::new(&helper.username, value),
            (None, Some(value)) => value.clone(),
            (None, None) => PasswordHash::invalid(),
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "'password' and 'password_hash' could not be used together",
                ));
            }
        };

        Ok(User {
            username: helper.username,
            password_hash,
            active: helper.active,
            creation_date: helper.creation_date,
            related_to: helper.related_to,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub identifier: PrimaryKey<User>,
    pub username: String,
    pub active: bool,
    pub creation_date: Date,
    pub related_to: Option<PrimaryKey<Person>>,
}

impl From<Record<User>> for Metadata {
    fn from(value: Record<User>) -> Self {
        let identifier = value.identifier;
        let value: User = value.value;
        Metadata {
            identifier,
            username: value.username,
            active: value.active,
            creation_date: value.creation_date,
            related_to: value.related_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PasswordHash, User};
    use crate::{
        database::{Database, DatabaseEntry, DefaultGenerator, Insertable, Record},
        Date,
    };

    #[test]
    fn test_hash() {
        let username = "Chris";
        let user = User {
            username: String::from(username),
            password_hash: PasswordHash::new("Chris", "test1234"),
            active: true,
            creation_date: Date::today(),
            related_to: None,
        };

        assert_eq!(user.password_hash.matches(username, "test123"), false);
        assert_eq!(user.password_hash.matches(username, "test1234"), true);
    }

    #[test]
    fn test_hash_after_insert() {
        let username = "Chris";

        let database = Database::plain().expect("valid database");
        User::create_table(&database).expect("valid table");

        let _ = User {
            username: String::from(username),
            password_hash: PasswordHash::new(username, "test1234"),
            active: true,
            creation_date: Date::today(),
            related_to: None,
        }
        .insert(&database)
        .expect("Insert sucessful");

        let user = User::select_by_name(&database, username)
            .expect("valid sample")
            .expect("existing value");
        assert_eq!(user.password_hash.matches(username, "test123"), false);
        assert_eq!(user.password_hash.matches(username, "test1234"), true);
    }

    #[test]
    fn test_select_by_name() {
        const USERNAME: &'static str = "Chris";
        let database = Database::in_memory().expect("valid database");

        // Create something in the database
        let user_id = {
            let mut user = User::create_default(&database);
            user.username = String::from(USERNAME);
            user.insert(&database).expect("sucessful insert")
        };

        let found_user = User::select_by_name(&database, USERNAME)
            .expect("select ok")
            .expect("existing record");
        assert_eq!(found_user.identifier, user_id);
    }

    #[test]
    fn test_select_by_name_non_existing() {
        const USERNAME: &'static str = "Chris";
        let database = Database::in_memory().expect("valid database");

        let _ = {
            let mut user = User::create_default(&database);
            user.username = String::from(USERNAME);
            user.insert(&database).expect("sucessful insert")
        };

        assert_eq!(
            User::select_by_name(&database, "Max").expect("select ok"),
            None
        )
    }

    #[test]
    fn test_serialization() {
        let user = User {
            username: "test_user".to_string(),
            password_hash: PasswordHash::new("test_user", "password"),
            active: true,
            creation_date: Date::today(),
            related_to: None,
        };

        let serialized = serde_json::to_string(&user).expect("serialization successful");
        let deserialized: User =
            serde_json::from_str(&serialized).expect("deserialization successful");

        assert_eq!(deserialized.username, user.username);
        assert!(deserialized.password_hash.is_valid());
        assert_eq!(deserialized.active, user.active);
        assert_eq!(deserialized.creation_date, user.creation_date);
        assert_eq!(deserialized.related_to, user.related_to);
    }

    #[test]
    fn test_deserialization_with_password() {
        let json_str = r#"
            {
                "username": "test_user",
                "password": "password",
                "active": true,
                "creation_date": "2024-02-18",
                "related_to": null
            }
        "#;

        let deserialized: User =
            serde_json::from_str(json_str).expect("deserialization successful");

        assert_eq!(deserialized.username, "test_user");
        assert!(deserialized.password_hash.matches("test_user", "password"));
        assert_eq!(deserialized.active, true);
        assert_eq!(deserialized.related_to, None);
    }

    #[test]
    fn test_deserialization_with_password_hash() {
        let json_str = r#"
            {
                "username": "test_user",
                "password_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "active": true,
                "creation_date": "2024-02-18",
                "related_to": null
            }
        "#;

        let deserialized: User =
            serde_json::from_str(json_str).expect("deserialization successful");

        assert_eq!(deserialized.username, "test_user");
        assert!(deserialized.password_hash.is_valid());
        assert_eq!(deserialized.active, true);
        assert_eq!(deserialized.related_to, None);
    }

    #[test]
    fn test_deserialization_record_list() {
        let json = r#"
        [
            {"identifier":"/users/1","username":"admin","password_hash":[14,57,8,8,122,249,218,134,181,69,76,200,167,57,21,106,176,131,229,57,123,93,169,66,225,125,87,8,27,55,57,151],"active":true,"creation_date":"2024-02-19","related_to":null},
            {"identifier":"/users/2","username":"Chris","password_hash":[177,77,191,48,246,167,199,122,115,155,85,45,122,141,19,103,67,204,211,167,222,82,245,220,136,138,197,79,164,70,178,170],"active":true,"creation_date":"2024-02-19","related_to":null}
        ]
        "#;

        let records: Vec<Record<User>> =
            serde_json::from_str(json).expect("deserialization successful");

        assert_eq!(records.len(), 2);
    }
}
