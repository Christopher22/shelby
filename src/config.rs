use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rocket::fs::NamedFile;
use shelby_backend::database::Database;

pub struct Config {
    database: Mutex<Database>,
    public_assets: PathBuf,
}

impl Config {
    pub const ENV_VARIBLE_PATH: &'static str = "SHELBY_ASSETS";

    pub fn from_env(database: Database) -> Option<Self> {
        let public_assets = std::env::var(Self::ENV_VARIBLE_PATH)
            .ok()
            .and_then(|value| {
                let path = PathBuf::from(value);
                match path.is_dir() {
                    true => Some(path),
                    false => None,
                }
            })?;

        Some(Config {
            database: Mutex::new(database),
            public_assets,
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
