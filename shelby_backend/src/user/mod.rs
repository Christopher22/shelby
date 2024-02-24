use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::database::{Database, DatabaseEntry, DefaultGenerator, PrimaryKey, Record, Selectable};
use crate::person::Person;
use crate::Date;

mod password_hash;
pub use self::password_hash::PasswordHash;

crate::database::make_struct!(
    User (Table with derived Serialize: "users") depends on Person => {
        username: String,
        password_hash: PasswordHash,
        active: bool,
        creation_date: Date,
        related_to: Option<PrimaryKey<Person>>
    } ("FOREIGN KEY(related_to) REFERENCES persons(id)")
);

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
                <Self as Selectable>::SelectValue::try_from(row).map(Self::deserialize_sql)
            })
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{PasswordHash, User};
    use crate::{
        database::{Database, DatabaseEntry, DefaultGenerator, Insertable, Record, Selectable},
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

        let index = User {
            username: String::from(username),
            password_hash: PasswordHash::new(username, "test1234"),
            active: true,
            creation_date: Date::today(),
            related_to: None,
        }
        .insert(&database)
        .expect("Insert sucessful");

        let user = User::select(&database, index).expect("valid sample");
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
