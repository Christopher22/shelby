use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::backend::database::Database;
use base64::prelude::*;
use rocket::fs::NamedFile;

pub struct Config {
    database: Mutex<Database>,
    public_assets: PathBuf,
    secret: [u8; 32],
}

impl Config {
    pub const ENV_VARIBLE_PATH: &'static str = "SHELBY_ASSETS";
    const ENV_SECRET: &'static str = "ROCKET_SECRET_KEY";

    pub fn from_env(database: Database) -> Result<Self, Error> {
        let public_assets = std::env::var(Self::ENV_VARIBLE_PATH)
            .or(Err(Error::AssetsNotFound))
            .and_then(|value| {
                let path = PathBuf::from(value);
                match path.is_dir() {
                    true => Ok(path),
                    false => Err(Error::AssetsNotFound),
                }
            })?;

        // Create a secret or use teh default one
        let secret = match std::env::var(Config::ENV_SECRET) {
            Ok(input) => BASE64_STANDARD
                .decode(input)
                .or(Err(Error::InvalidSecretKey))
                .and_then(|v| v.try_into().or(Err(Error::InvalidSecretKey)))?,
            Err(_) => {
                let mut secret: [u8; 32] = [0u8; 32];
                getrandom::getrandom(&mut secret).or(Err(Error::RandomNotAvailable))?;
                std::env::set_var(Config::ENV_SECRET, BASE64_STANDARD.encode(&secret));
                secret
            }
        };

        Ok(Config {
            database: Mutex::new(database),
            public_assets,
            secret,
        })
    }

    /// Get a (safe) NamedFile for a public asset.
    pub fn send_asset(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = Result<NamedFile, std::io::Error>> {
        NamedFile::open(self.public_assets.join(path))
    }

    /// Get a handle to the database.
    pub fn database(&self) -> std::sync::MutexGuard<'_, Database> {
        self.database.lock().expect("database mutex")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    AssetsNotFound,
    RandomNotAvailable,
    InvalidSecretKey,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AssetsNotFound => write!(
                f,
                "env variable {} does not point to valid asset directory",
                Config::ENV_VARIBLE_PATH
            ),
            Error::RandomNotAvailable => f.write_str("unable to get random data for secret key"),
            Error::InvalidSecretKey => f.write_str("the specified secret key is invalid"),
        }
    }
}
