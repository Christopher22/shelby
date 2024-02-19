use std::num::NonZeroU32;

use rusqlite::types::ValueRef;

static PBKDF2_ALGORITHM: ring::pbkdf2::Algorithm = ring::pbkdf2::PBKDF2_HMAC_SHA256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordHash([u8; Self::CREDENTIAL_LEN], bool);

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
        Self(credential, true)
    }

    /// Create an invalid password hash.
    pub fn invalid() -> Self {
        Self(Default::default(), false)
    }

    pub fn matches(&self, username: &str, other_password: &str) -> bool {
        if !self.is_valid() {
            return false;
        }

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

    pub fn is_valid(&self) -> bool {
        self.1
    }

    fn salt(username: &str) -> Vec<u8> {
        let mut salt = Vec::with_capacity(Self::FIXED_SALT.len() + username.as_bytes().len());
        salt.extend(Self::FIXED_SALT.as_ref());
        salt.extend(username.as_bytes());
        salt
    }
}

impl serde::Serialize for PasswordHash {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PasswordHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <[u8; Self::CREDENTIAL_LEN]>::deserialize(deserializer)
            .map(|value| PasswordHash(value, true))
    }
}

impl rusqlite::ToSql for PasswordHash {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        // Do not store invalid hashes in the database.
        if !self.is_valid() {
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                PasswordHashError,
            )));
        }

        Ok(rusqlite::types::ToSqlOutput::Borrowed(ValueRef::Blob(
            &self.0,
        )))
    }
}

impl rusqlite::types::FromSql for PasswordHash {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        <[u8; Self::CREDENTIAL_LEN]>::column_result(value).map(|value| PasswordHash(value, true))
    }
}

impl crate::database::DatabaseType for PasswordHash {
    const RAW_COLUMN_VALUE: &'static str = "BLOB";
    const COLUMN_VALUE: &'static str = "BLOB NOT NULL";
}

/// The error when somebody tries to insert an invalid password hash into the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PasswordHashError;

impl std::fmt::Display for PasswordHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the password hash is invalid and could not be added to the database"
        )
    }
}

impl std::error::Error for PasswordHashError {}
