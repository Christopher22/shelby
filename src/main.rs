#![allow(non_snake_case)] // Required due to https://github.com/rwf2/Rocket/issues/1003

#[macro_use]
extern crate rocket;

mod auth;
mod config;
mod error;
mod frontend;
mod util;

use auth::{login, logout, AuthenticatedUser};
use rocket::data::{Limits, ToByteUnit};
use rocket::{fs::NamedFile, serde::json::Json, State};
use rocket_dyn_templates::{context, Template};
use shelby_backend::database::{
    Database, DefaultGenerator, Insertable, PrimaryKey, SelectableByPrimaryKey,
};
use std::path::PathBuf;

pub use self::frontend::{InsertableDatabaseEntry, Renderable, RenderableDatabaseEntry};
pub use self::util::{FlexibleInput, PdfOutput};
pub use self::{config::Config, error::Error};

macro_rules! create_routes {
    ($database_entry: ty {
        module: $function_name: ident,
        url: $path: literal,
        get: $path_id: literal,
        post: $path_add: literal
    }) => {
        mod $function_name {
            use rocket::{response::status, serde::json::Json, State};
            use rocket_dyn_templates::Template;
            use shelby_backend::database::{Insertable, Selectable};

            use crate::{
                auth::AuthenticatedUser,
                frontend::{InsertableDatabaseEntry, Renderable, RenderableDatabaseEntry},
                *,
            };

            type DatabaseEntry = $database_entry;
            type InputType = <$database_entry as InsertableDatabaseEntry>::PostMethod;

            #[post($path, data = "<database_entry>", rank = 3)]
            pub fn add(
                _user: AuthenticatedUser,
                database_entry: InputType,
                state: &State<Config>,
            ) -> Result<status::Created<String>, Error> {
                database_entry
                    .into_inner()
                    .insert(&state.database())
                    .map(|primary_key| status::Created::new(primary_key.to_string()))
                    .map_err(Error::from)
            }

            #[get($path_add, rank = 2)]
            pub fn add_frontend(user: AuthenticatedUser) -> Template {
                DatabaseEntry::prepare_rendering($path, user).render()
            }

            #[get($path, rank = 3)]
            pub fn get_all(
                _user: AuthenticatedUser,
                state: &State<Config>,
                content_type: Option<&rocket::http::ContentType>,
            ) -> Result<Result<Template, Json<Vec<<DatabaseEntry as Selectable>::Output>>>, Error>
            {
                let database = &state.database();
                Ok(match content_type {
                    Some(value) if value.0.is_json() => {
                        Err(Json(<$database_entry>::select_all(&database)?))
                    }
                    _ => Ok(<$database_entry>::prepare_rendering_all(&database)?.render()),
                })
            }

            #[get($path_id, rank = 3)]
            pub fn get_by_id(
                _user: AuthenticatedUser,
                id: i64,
                state: &State<Config>,
            ) -> Result<Json<<DatabaseEntry as Selectable>::Output>, Error> {
                match DatabaseEntry::try_select(&state.database(), id)? {
                    Some(value) => Ok(Json(value)),
                    None => Err(Error::NotFound),
                }
            }

            #[cfg(test)]
            mod tests {
                use crate::frontend::RenderableDatabaseEntry;
                use crate::{rocket, Config};
                use rocket::{http::Status, local::blocking::Client, serde::json, State};
                use shelby_backend::database::{DefaultGenerator, PrimaryKey, Record, Selectable};

                use super::DatabaseEntry as TargetEntity;
                const ACCESS_POINT: &'static str = $path;

                #[test]
                fn test_get_empty() {
                    let client = crate::tests::login(rocket());
                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                    assert_eq!(
                        response.content_type(),
                        Some(rocket::http::ContentType::HTML)
                    );

                    // Ensure the reponse contains the title
                    let response = response.into_string().expect("valid str");
                    assert!(response.find(TargetEntity::TITLE).is_some());
                }

                #[test]
                fn test_get_empty_json() {
                    // We need to check the number of elements here. For example, the users table will never be empty
                    let (client, num_elements) =
                        crate::tests::login_with_callback(rocket(), |database| {
                            TargetEntity::select_all(database)
                                .expect("selecting all successfull")
                                .len()
                        });
                    let response = client
                        .get(ACCESS_POINT)
                        .header(rocket::http::ContentType::JSON)
                        .dispatch();
                    assert_eq!(response.status(), Status::Ok);
                    assert_eq!(
                        response.content_type(),
                        Some(rocket::http::ContentType::JSON)
                    );

                    let response = response.into_string().expect("valid str");
                    let response_json: Vec<Record<TargetEntity>> =
                        json::from_str(&response).expect("valid json");
                    assert_eq!(response_json.len(), num_elements);
                }

                #[test]
                fn test_get_all() {
                    let engine = rocket();
                    let example = {
                        let state: &State<Config> = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&state.database())
                    };

                    let client = crate::tests::login(engine);
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Created);

                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Ok);
                    assert_eq!(
                        response.content_type(),
                        Some(rocket::http::ContentType::HTML)
                    );

                    // Ensure the reponse contains the title
                    let response = response.into_string().expect("valid str");
                    assert!(response.find(TargetEntity::TITLE).is_some());
                }

                #[test]
                fn test_get_all_json() {
                    let engine = rocket();
                    let example = {
                        let state: &State<Config> = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&state.database())
                    };

                    let (client, num_elements) =
                        crate::tests::login_with_callback(engine, |database| {
                            TargetEntity::select_all(database)
                                .expect("selecting all successfull")
                                .len()
                        });
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Created);

                    let response = client
                        .get(ACCESS_POINT)
                        .header(rocket::http::ContentType::JSON)
                        .dispatch();
                    assert_eq!(response.status(), Status::Ok);
                    assert_eq!(
                        response.content_type(),
                        Some(rocket::http::ContentType::JSON)
                    );

                    // Why does this fail?
                    // let response_json: Vec<Record<TargetEntity>> = response.into_json().expect("valid json");
                    let response = response.into_string().expect("valid str");
                    let response_json: Vec<<TargetEntity as Selectable>::Output> =
                        json::from_str(&response).expect("valid json");
                    assert_eq!(response_json.len(), num_elements + 1);
                }

                #[test]
                fn test_get_by_id() {
                    let engine = rocket();
                    let example = {
                        let state: &State<Config> = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&state.database())
                    };

                    let client = crate::tests::login(engine);
                    let creation_response = client.post(ACCESS_POINT).json(&example).dispatch();
                    assert_eq!(creation_response.status(), Status::Created);

                    // Extract the primary key
                    let primary_key_path = creation_response
                        .headers()
                        .get_one("Location")
                        .expect("valid string");
                    let response = client.get(primary_key_path).dispatch();
                    assert_eq!(response.status(), Status::Ok);

                    // Parse the Json response
                    let response = response.into_string().expect("valid str");
                    let response_json: <TargetEntity as Selectable>::Output =
                        json::from_str(&response).expect("valid json");

                    assert_eq!(
                        response_json,
                        <TargetEntity as Selectable>::Output::from(Record {
                            identifier: <PrimaryKey<_> as std::str::FromStr>::from_str(
                                primary_key_path
                            )
                            .expect("valid key"),
                            value: example
                        })
                    );
                }

                #[test]
                fn test_get_by_id_not_found() {
                    let client = crate::tests::login(rocket());
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
                    let client = crate::tests::login(rocket());

                    let login_response = client.get("/users/logout").dispatch();
                    assert_eq!(login_response.status(), rocket::http::Status::SeeOther);

                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Unauthorized);
                }
            }
        }
    };
}

macro_rules! write_routes {
    ($($function_name: ident),* + ($($additional: ident),*)) => { paste::paste! {
        routes![$($additional),*, $(
            $function_name::add, $function_name::get_all, $function_name::get_by_id, $function_name::add_frontend
        ),*]
    }};
}

// ------------------- Routes -------------------

#[get("/", rank = 1)]
async fn index_protected(_user: AuthenticatedUser<auth::Forward>) -> Template {
    Template::render("dashboard", context! {})
}

#[get("/", rank = 2)]
async fn index_public() -> Template {
    Template::render("login", context! {})
}

#[get("/<file..>", rank = 10)]
async fn serve_files(file: PathBuf, config: &State<Config>) -> Option<NamedFile> {
    config.send_asset(file).await.ok()
}

#[catch(default)]
async fn error_handler(
    status: rocket::http::Status,
    req: &rocket::Request<'_>,
) -> Result<Template, Json<()>> {
    match req.content_type() {
        Some(value) if value.0.is_json() => Err(Json(())),
        _ => Ok(Template::render(
            "error",
            context! { error: status.reason_lossy() },
        )),
    }
}

create_routes!(shelby_backend::person::Person {
    module: person,
    url: "/persons",
    get: "/persons/<id>",
    post: "/persons/new"
});

create_routes!(shelby_backend::person::Group {
    module: group,
    url: "/groups",
    get: "/groups/<id>",
    post: "/groups/new"
});

create_routes!(shelby_backend::document::Document {
    module: document,
    url: "/documents",
    get: "/documents/<id>",
    post: "/documents/new"
});

#[get("/documents/<id>/pdf")]
async fn download_document(
    id: i64,
    state: &State<Config>,
    _user: AuthenticatedUser,
) -> Result<PdfOutput, Error> {
    PdfOutput::new(&state.database(), PrimaryKey::from(id))
}

create_routes!(shelby_backend::user::User {
    module: user,
    url: "/users",
    get: "/users/<id>",
    post: "/users/new"
});

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

    let limits = Limits::default()
        .limit("form", 5.mebibytes())
        .limit("file", 5.mebibytes())
        .limit("data-form", 5.mebibytes())
        .limit("bytes", 5.mebibytes());

    let figment = rocket::Config::figment().merge(
        rocket::figment::providers::Serialized::defaults(rocket::Config {
            secret_key: rocket::config::SecretKey::generate().expect("safe RNG available"),
            limits,
            ..Default::default()
        }),
    );

    rocket::custom(figment)
        .manage(config)
        .attach(Template::fairing())
        .register("/", catchers![error_handler])
        .mount(
            "/",
            write_routes!(
                person,
                group,
                document,
                user + (
                    index_protected,
                    index_public,
                    serve_files,
                    login,
                    logout,
                    download_document
                )
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::{auth, rocket, Config};
    use rocket::{http::ContentType, local::blocking::Client, State};
    use rocket_dyn_templates::context;
    use shelby_backend::database::{DefaultGenerator, Insertable};

    fn add_user<P: rocket::Phase>(
        engine: rocket::Rocket<P>,
        credentials: &auth::Credentials,
    ) -> Client {
        let (client, _) = add_user_with_callback(engine, credentials, |_| ());
        client
    }

    fn add_user_with_callback<P: rocket::Phase, T>(
        engine: rocket::Rocket<P>,
        credentials: &auth::Credentials,
        callback: impl Fn(&shelby_backend::database::Database) -> T,
    ) -> (Client, T) {
        let result = {
            let database_container: &State<Config> = State::get(&engine).expect("valid database");
            let database = database_container.database();

            let mut user = shelby_backend::user::User::create_default(&database);
            user.username = String::from(&credentials.user);
            user.password_hash =
                shelby_backend::user::PasswordHash::new(&credentials.user, &credentials.password);
            user.insert(&database).expect("user insertion sucessfull");

            callback(&database)
        };

        (Client::tracked(engine).expect("valid client"), result)
    }

    /// Compare a recieved response with a template.
    fn compare_response(client: &Client, url: &'static str, template: &'static str) {
        let recieved_response = client
            .get(url)
            .dispatch()
            .into_string()
            .expect("valid string");

        let expected_output =
            rocket_dyn_templates::Template::show(client.rocket(), template, context! {})
                .expect("valid output");

        assert_eq!(recieved_response, expected_output);
    }

    pub fn login_with_callback<P: rocket::Phase, T>(
        engine: rocket::Rocket<P>,
        callback: impl Fn(&shelby_backend::database::Database) -> T,
    ) -> (Client, T) {
        let credentials = auth::Credentials {
            user: String::from("Chris"),
            password: String::from("test1234"),
        };

        let (client, result) = add_user_with_callback(engine, &credentials, callback);

        // Log in
        {
            let creation_response = client
                .post("/users/login")
                .header(ContentType::Form)
                .body("user=Chris&password=test1234")
                .dispatch();
            assert_eq!(creation_response.status(), rocket::http::Status::SeeOther);
        }

        (client, result)
    }

    pub fn login<P: rocket::Phase>(engine: rocket::Rocket<P>) -> Client {
        let (client, _) = login_with_callback(engine, |_| ());
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
        compare_response(&client, "/", "login");

        // Simulate login
        let creation_response = client
            .post("/users/login")
            .header(ContentType::Form)
            .body("user=Chris&password=test1234")
            .dispatch();
        assert_eq!(creation_response.status(), rocket::http::Status::SeeOther);

        // Now we get the dashboard!
        compare_response(&client, "/", "dashboard");
    }

    #[test]
    fn test_error_catching_html() {
        let client = Client::tracked(rocket()).expect("valid client");
        let response = client.get("/invalid_page").dispatch();

        assert_eq!(response.status(), rocket::http::Status::NotFound);

        // Ensure we got a custom error page mentioning "Shelby"
        let response = response.into_string().expect("valid string");
        assert!(response.find("Shelby").is_some())
    }

    #[test]
    fn test_error_catching_json() {
        let client = Client::tracked(rocket()).expect("valid client");
        let response = client
            .get("/invalid_page")
            .header(rocket::http::ContentType::JSON)
            .dispatch();

        assert_eq!(response.status(), rocket::http::Status::NotFound);
        assert_eq!(
            response.content_type(),
            Some(rocket::http::ContentType::JSON)
        );

        let response = response.into_string().expect("valid str");
        let _: () = rocket::serde::json::from_str(&response).expect("valid json");
    }

    #[test]
    fn test_document_pdf() {
        let engine = rocket();
        let example_data = vec![42u8, 41, 40];
        let example = {
            let state: &State<Config> = State::get(&engine).expect("valid database");

            let mut example = shelby_backend::document::Document::create_default(&state.database());
            example.document = example_data.clone();
            example
        };

        let client = crate::tests::login(engine);
        let creation_response = client.post("/documents").json(&example).dispatch();
        assert_eq!(creation_response.status(), rocket::http::Status::Created);

        // Generate the URL for the PDF
        let pdf_url = format!(
            "{}/pdf",
            creation_response
                .headers()
                .get_one("Location")
                .expect("valid string")
        );

        let response = client.get(pdf_url).dispatch();
        // Check that the PDF should be viewed in the browser
        assert_eq!(
            response.headers().get_one("Content-Disposition"),
            Some("inline")
        );
        assert_eq!(response.content_type(), Some(ContentType::PDF));
        assert_eq!(response.into_bytes().expect("valid bytes"), example_data);
    }
}
