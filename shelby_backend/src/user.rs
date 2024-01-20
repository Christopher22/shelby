use std::num::NonZeroU32;

use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};

use crate::person::Person;
use crate::PrimaryKey;

static PBKDF2_ALGORITHM: ring::pbkdf2::Algorithm = ring::pbkdf2::PBKDF2_HMAC_SHA256;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct PasswordHash([u8; Self::CREDENTIAL_LEN]);

impl PasswordHash {
    const PBKDF_ITERATIONS: NonZeroU32 = match NonZeroU32::new(100_000) {
        Some(v) => v,
        #[allow(unconditional_panic)]
        None => [][0],
    };

    const FIXED_SALT: [u8; 5] = [67, 104, 114, 105, 115];
    const CREDENTIAL_LEN: usize = ring::digest::SHA256_OUTPUT_LEN;

    pub fn new(username: &str, password: &str) -> Self {
        let salt = PasswordHash::salt(username);
        let mut credential = [0u8; Self::CREDENTIAL_LEN];
        ring::pbkdf2::derive(
            PBKDF2_ALGORITHM,
            Self::PBKDF_ITERATIONS,
            &salt,
            password.as_bytes(),
            &mut credential,
        );
        Self(credential)
    }

    pub fn matches(&self, username: &str, other_password: &str) -> bool {
        let salt = Self::salt(username);
        ring::pbkdf2::verify(
            PBKDF2_ALGORITHM,
            Self::PBKDF_ITERATIONS,
            &salt,
            other_password.as_bytes(),
            &self.0,
        )
        .is_ok()
    }

    fn salt(username: &str) -> Vec<u8> {
        let mut salt = Vec::with_capacity(Self::FIXED_SALT.len() + username.as_bytes().len());
        salt.extend(Self::FIXED_SALT.as_ref());
        salt.extend(username.as_bytes());
        salt
    }
}

impl rusqlite::ToSql for PasswordHash {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Borrowed(ValueRef::Blob(
            &self.0,
        )))
    }
}

impl rusqlite::types::FromSql for PasswordHash {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        <[u8; Self::CREDENTIAL_LEN]>::column_result(value).map(PasswordHash)
    }
}

crate::macros::make_struct!(
    User (Table: "users") depends on Person => {
        username: String => "STRING NOT NULL",
        password_hash: PasswordHash => "BLOB NOT NULL",
        active: bool => "BOOL NOT NULL",
        creation_date: chrono::DateTime<chrono::Utc> => "DATETIME NOT NULL",
        related_to: Option<PrimaryKey<Person>> => "INTEGER"
    } ("FOREIGN KEY(related_to) REFERENCES persons(id)")
);

#[cfg(test)]
mod tests {
    use super::{PasswordHash, User};
    use crate::{Database, DatabaseEntry, IndexableDatebaseEntry};

    #[test]
    fn test_hash() {
        let username = "Chris";
        let user = User {
            username: String::from(username),
            password_hash: PasswordHash::new("Chris", "test1234"),
            ..Default::default()
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
            password_hash: PasswordHash::new("Chris", "test1234"),
            ..Default::default()
        }
        .insert(&database)
        .expect("Insert sucessful");

        let user = User::select(&database, index).expect("valid sample");
        assert_eq!(user.password_hash.matches(username, "test123"), false);
        assert_eq!(user.password_hash.matches(username, "test1234"), true);
    }
}
