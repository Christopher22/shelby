#![allow(non_snake_case)] // Required due to https://github.com/rwf2/Rocket/issues/1003

#[macro_use]
extern crate rocket;

mod auth;
mod error;

use auth::{login, logout, AuthenticatedUser};
use rocket::{fs::NamedFile, response::status, serde::json::Json, State};
use shelby_backend::{
    document::Document,
    person::{Group, Person},
    Database, DefaultGenerator, IndexableDatebaseEntry, Record,
};
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub use self::error::Error;

struct Config {
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
                match path.join("dashboard.html").is_file() && path.join("public").is_dir() {
                    true => Some(path.join("public")),
                    false => None,
                }
            })?;

        Some(Config {
            database: Mutex::new(database),
            public_assets,
        })
    }

    /// Get a (safe) NamedFile for a public asset.
    fn private_assets(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = Result<NamedFile, std::io::Error>> {
        NamedFile::open(
            self.public_assets
                .parent()
                .expect("valid parent")
                .join(path),
        )
    }

    /// Get a (safe) NamedFile for a public asset.
    fn public_assets(
        &self,
        path: impl AsRef<Path>,
    ) -> impl Future<Output = Result<NamedFile, std::io::Error>> {
        NamedFile::open(self.public_assets.join(path))
    }

    /// Get a handle to the database.
    fn database(&self) -> std::sync::MutexGuard<'_, Database> {
        self.database.lock().expect("database mutex")
    }
}

macro_rules! create_routes {
    ($path: literal ($path_id: literal) => $database_entry: ident ($function_name: ident)) => {
        paste::paste! {
            #[post($path, format = "json", data = "<database_entry>", rank = 3)]
            fn [< add_ $function_name >](
                _user: AuthenticatedUser,
                database_entry: Json<$database_entry>,
                state: &State<Config>,
            ) -> Result<status::Created<String>, Error> {
                database_entry
                    .0
                    .insert(&state.database())
                    .map(|primary_key| status::Created::new(primary_key.to_string()))
                    .map_err(Error::from)
            }

            #[get($path, rank = 3)]
            fn [< get_all_ $function_name s >](_user: AuthenticatedUser, state: &State<Config>) -> Result<Json<Vec<Record<$database_entry>>>, Error> {
                Ok(Json($database_entry::select_all(
                    &state.database()
                )?))
            }

            #[get($path_id, rank = 3)]
            fn [< get_ $function_name _by_id>](_user: AuthenticatedUser, id: i64, state: &State<Config>) -> Result<Json<Record<$database_entry>>, Error> {
                match $database_entry::try_select(
                    &state.database(),
                    id
                )? {
                    Some(value) => Ok(Json(value)),
                    None => Err(Error::NotFound)
                }
            }

            #[cfg(test)]
            mod [< test_ $function_name >] {
                use super::{rocket, Config};
                use rocket::{http::Status, local::blocking::Client, serde::json, State};
                use shelby_backend::{DefaultGenerator, Record};

                type TargetEntity = super::$database_entry;
                const ACCESS_POINT: &'static str = $path;

                #[test]
                fn test_get_empty() {
                    let client = super::tests::login(rocket());
                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Ok);

                    let response_json: Vec<Record<TargetEntity>> = response.into_json().expect("valid json");
                    assert_eq!(response_json.len(), 0);
                }

                #[test]
                fn test_get() {
                    let engine = rocket();
                    let example = {
                        let state: &State<Config> = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&state.database())
                    };

                    let client = super::tests::login(engine);
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Created);

                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Ok);

                    // Why does this fail?
                    // let response_json: Vec<Record<TargetEntity>> = response.into_json().expect("valid json");
                    let response = response.into_string().expect("valid str");
                    let response_json: Vec<Record<TargetEntity>> = json::from_str(&response).expect("valid json");
                    assert_eq!(response_json.len(), 1);
                }

                #[test]
                fn test_get_by_id() {
                    let engine = rocket();
                    let example = {
                        let state: &State<Config> = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&state.database())
                    };

                    let client = super::tests::login(engine);
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Created);

                    let primary_key_path = creation_response.headers().get_one("Location").expect("valid string");
                    let response = client.get(primary_key_path).dispatch();
                    assert_eq!(response.status(), Status::Ok);

                    let response = response.into_string().expect("valid str");
                    let response_json: Record<TargetEntity> = json::from_str(&response).expect("valid json");
                    assert_eq!(*response_json, example);
                }


                #[test]
                fn test_get_by_id_not_found() {
                    let client = super::tests::login(rocket());
                    let response = client.get(format!("{}/42", ACCESS_POINT)).dispatch();
                    assert_eq!(response.status(), Status::NotFound);
                }

                #[test]
                fn test_get_empty_unauthorized() {
                    let client = Client::tracked(rocket()).expect("valid client");
                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Unauthorized);
                }

                #[test]
                fn test_get_unauthorized() {
                    let engine = rocket();
                    let state: &State<Config> = State::get(&engine).expect("valid database");
                    let example = TargetEntity::create_default(&state.database());

                    let client = Client::tracked(engine).expect("valid client");
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Unauthorized);
                }

                #[test]
                fn test_get_by_id_non_existing_unauthorized() {
                    let engine = rocket();
                    let client = Client::tracked(engine).expect("valid client");

                    let response = client.get(format!("{}/42", ACCESS_POINT)).dispatch();
                    assert_eq!(response.status(), Status::Unauthorized);
                }

                #[test]
                fn test_logout() {
                    let client = super::tests::login(rocket());

                    let login_response = client.get("/users/logout").dispatch();
                    assert_eq!(login_response.status(), rocket::http::Status::SeeOther);

                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Unauthorized);
                }
            }
        }
    }
}

macro_rules! write_routes {
    ($($function_name: ident),* + ($($additional: ident),*)) => { paste::paste! {
        routes![$($additional),*, $(
            [< add_ $function_name >], [< get_all_ $function_name s >], [< get_ $function_name _by_id>]
        ),*]
    }};
}

create_routes!("/persons" ("/persons/<id>") => Person (person));
create_routes!("/groups" ("/groups/<id>") => Group (group));
create_routes!("/documents" ("/documents/<id>") => Document (document));

#[get("/", rank = 1)]
async fn index_protected(
    _user: AuthenticatedUser<auth::Forward>,
    config: &State<Config>,
) -> Option<NamedFile> {
    config.private_assets("dashboard.html").await.ok()
}

#[get("/", rank = 2)]
async fn index_public(config: &State<Config>) -> Option<NamedFile> {
    config.private_assets("login.html").await.ok()
}

#[get("/<file..>", rank = 10)]
async fn serve_files(file: PathBuf, config: &State<Config>) -> Option<NamedFile> {
    config.public_assets(file).await.ok()
}

#[launch]
fn rocket() -> _ {
    let database = Database::in_memory().expect("valid database");

    // Add a first default user
    {
        let mut admin = shelby_backend::user::User::create_default(&database);
        admin.username = String::from("admin");
        admin.password_hash = shelby_backend::user::PasswordHash::new("admin", "test1234");
        admin.insert(&database).expect("unable to add Admin user");
    }

    let config = match Config::from_env(database) {
        Some(value) => value,
        None => {
            eprintln!(
                "Env variable {} does not point to valid asset directory",
                Config::ENV_VARIBLE_PATH
            );
            std::process::exit(1)
        }
    };

    let figment = rocket::Config::figment().merge(
        rocket::figment::providers::Serialized::defaults(rocket::Config {
            secret_key: rocket::config::SecretKey::generate().expect("safe RNG available"),
            ..Default::default()
        }),
    );

    rocket::custom(figment).manage(config).mount(
        "/",
        write_routes!(
            person,
            group,
            document + (index_protected, index_public, serve_files, login, logout)
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::{auth, rocket, Config};
    use rocket::{http::ContentType, local::blocking::Client, State};
    use shelby_backend::{DefaultGenerator, IndexableDatebaseEntry};

    fn add_user<P: rocket::Phase>(
        engine: rocket::Rocket<P>,
        credentials: &auth::Credentials,
    ) -> Client {
        let _ = {
            let database_container: &State<Config> = State::get(&engine).expect("valid database");
            let database = database_container.database();

            let mut user = shelby_backend::user::User::create_default(&database);
            user.username = String::from(&credentials.user);
            user.password_hash =
                shelby_backend::user::PasswordHash::new(&credentials.user, &credentials.password);
            user.insert(&database).expect("user insertion sucessfull")
        };

        Client::tracked(engine).expect("valid client")
    }

    /// Compare a recieved response with a local file.
    fn compare_response(client: &Client, url: &str, asset_relative_path: &str) {
        let ASSET_FOLDER = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let recieved_response = client
            .get(url)
            .dispatch()
            .into_string()
            .expect("valid string");

        let mut expected_output = String::new();
        std::fs::File::open(ASSET_FOLDER.join(asset_relative_path))
            .expect("valid file")
            .read_to_string(&mut expected_output)
            .expect("read failed");

        assert_eq!(recieved_response, expected_output);
    }

    pub fn login<P: rocket::Phase>(engine: rocket::Rocket<P>) -> Client {
        let credentials = auth::Credentials {
            user: String::from("Chris"),
            password: String::from("test1234"),
        };

        let client = add_user(engine, &credentials);

        // Log in
        {
            let creation_response = client
                .post("/users/login")
                .header(ContentType::Form)
                .body("user=Chris&password=test1234")
                .dispatch();
            assert_eq!(creation_response.status(), rocket::http::Status::SeeOther);
        }

        client
    }

    #[test]
    fn test_login() {
        login(rocket());
    }

    #[test]
    fn test_login_fail() {
        let credentials = auth::Credentials {
            user: String::from("Chris"),
            password: String::from("test1234"),
        };
        let client = add_user(rocket(), &credentials);

        let creation_response = client
            .post("/users/login")
            .header(ContentType::Form)
            .body("user=Chris&password=WRONG_PASSWORD")
            .dispatch();

        // Check the wrong password is detected ...
        assert_eq!(
            creation_response.status(),
            rocket::http::Status::Unauthorized
        );

        // ... and the user is not logged in.
        let response = client.get("/persons").dispatch();
        assert_eq!(response.status(), rocket::http::Status::Unauthorized);
    }

    #[test]
    fn test_logout_without_login() {
        let client = Client::tracked(rocket()).expect("valid client");
        let login_response = client.get("/users/logout").dispatch();
        // We may thing about changing that
        assert_eq!(login_response.status(), rocket::http::Status::SeeOther);
    }

    #[test]
    fn test_login_page() {
        let credentials = auth::Credentials {
            user: String::from("Chris"),
            password: String::from("test1234"),
        };

        let client = add_user(rocket(), &credentials);

        // Check we receive first the login page
        compare_response(&client, "/", "assets/login.html");

        // Simulate login
        let creation_response = client
            .post("/users/login")
            .header(ContentType::Form)
            .body("user=Chris&password=test1234")
            .dispatch();
        assert_eq!(creation_response.status(), rocket::http::Status::SeeOther);

        // Now we get the dashboard!
        compare_response(&client, "/", "assets/dashboard.html");
    }
}
