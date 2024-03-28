#![allow(non_snake_case)] // Required due to https://github.com/rwf2/Rocket/issues/1003
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
extern crate rocket;

mod auth;
mod backend;
mod config;
mod error;
mod frontend;
mod util;

use backend::database::Selectable;
use rocket::{
    data::{Limits, ToByteUnit},
    form::Strict,
    fs::NamedFile,
    serde::json::Json,
    State,
};
use rocket_dyn_templates::{context, Template};
use std::ops::Deref;
use std::path::PathBuf;

use self::auth::{login, login_html, logout, AuthenticatedUser};
use self::backend::{
    database::{Database, DefaultGenerator, Insertable, PrimaryKey, SelectableByPrimaryKey},
    Pagination,
};
pub use self::frontend::{InsertableDatabaseEntry, Renderable, RenderableDatabaseEntry};
pub use self::util::{FlexibleInput, PdfOutput};
pub use self::{
    config::Config,
    error::{error_handler, Error},
};

macro_rules! create_routes {
    ($database_entry: ty {
        module: $function_name: ident,
        add_json: $path: literal,
        add_frontend: $path_add: literal,
        get_single: $path_id: literal,
        get_multiple: $path_multiple: literal
    }) => {
        mod $function_name {
            use crate::backend::database::{Insertable, Selectable};
            use rocket::{response::status, serde::json::Json, State};
            use rocket_dyn_templates::Template;

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
            pub fn add_frontend(user: AuthenticatedUser, state: &State<Config>) -> Template {
                let database_entry = state.database();
                DatabaseEntry::prepare_rendering($path, database_entry.deref(), user).render()
            }

            #[get($path_multiple, rank = 3)]
            pub fn get_all(
                _user: AuthenticatedUser,
                state: &State<Config>,
                content_type: Option<&rocket::http::ContentType>,

                limit: Option<crate::backend::Limit>,
                offset: Option<usize>,
                order: Option<crate::backend::Order>,
            ) -> Result<Result<Template, Json<Vec<<DatabaseEntry as Selectable>::Output>>>, Error>
            {
                // For some reason, putting pagination directly does not work. We generate it manually.
                let pagination = Pagination {
                    limit: limit.unwrap_or_default(),
                    column: crate::backend::Column::default(),
                    order: order.unwrap_or_default(),
                    offset: offset.unwrap_or(0),
                };
                let database = &state.database();

                Ok(match content_type {
                    Some(value) if value.0.is_json() => {
                        Err(Json(<$database_entry>::select_all_sorted(
                            &database, pagination, /*.into_inner()*/
                        )?))
                    }
                    _ => Ok(<$database_entry>::prepare_rendering_all(
                        &database, pagination, //.into_inner(),
                    )?
                    .render()),
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
                use crate::backend::database::{
                    DefaultGenerator, Insertable, PrimaryKey, Record, Selectable,
                };
                use crate::frontend::RenderableDatabaseEntry;
                use crate::{rocket, Config};
                use rocket::{http::Status, local::blocking::Client, serde::json, State};

                use super::DatabaseEntry as TargetEntity;
                const ACCESS_POINT: &'static str = $path;

                fn load_json(
                    client: &rocket::local::blocking::Client,
                    url: impl AsRef<str>,
                ) -> Vec<<TargetEntity as Selectable>::Output> {
                    let response = client
                        .get(url.as_ref())
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
                    json::from_str(&response).expect("valid json")
                }

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

                    let response_json = load_json(&client, ACCESS_POINT);
                    assert_eq!(response_json.len(), num_elements + 1);
                }

                #[test]
                fn test_get_all_json_limit() {
                    let client = {
                        let engine = rocket();
                        let state: &State<Config> = State::get(&engine).expect("valid database");

                        // Insert some entities
                        for _ in 0..4 {
                            let database = &state.database();
                            TargetEntity::create_default(database)
                                .insert(database)
                                .expect("valid insert");
                        }

                        crate::tests::login(engine)
                    };

                    let response_json = load_json(&client, format!("{}?limit=1", ACCESS_POINT));
                    assert_eq!(response_json.len(), 1);
                }

                #[test]
                fn test_get_all_json_order() {
                    let client = {
                        let engine = rocket();
                        let state: &State<Config> = State::get(&engine).expect("valid database");

                        // Insert some entities
                        for _ in 0..4 {
                            let database = &state.database();
                            TargetEntity::create_default(database)
                                .insert(database)
                                .expect("valid insert");
                        }

                        crate::tests::login(engine)
                    };

                    let mut results = Vec::new();
                    for value in ["asc", "desc"] {
                        results.push(load_json(
                            &client,
                            format!("{}?order={}&limit=1", ACCESS_POINT, value),
                        ));
                    }

                    assert_ne!(results[0], results[1]);
                }

                #[test]
                fn test_get_all_json_offset() {
                    let client = {
                        let engine = rocket();
                        let state: &State<Config> = State::get(&engine).expect("valid database");

                        // Insert some entities
                        for _ in 0..4 {
                            let database = &state.database();
                            TargetEntity::create_default(database)
                                .insert(database)
                                .expect("valid insert");
                        }

                        crate::tests::login(engine)
                    };

                    let response_all = load_json(&client, ACCESS_POINT);
                    let response_offset = load_json(&client, format!("{}?offset=1", ACCESS_POINT));
                    assert_eq!(response_all.len() - 1, response_offset.len());
                    assert_eq!(&response_all[1..], &response_offset);
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
                    assert_eq!(creation_response.status(), Status::Created, "post");

                    // Extract the primary key
                    let primary_key_path = creation_response
                        .headers()
                        .get_one("Location")
                        .expect("valid string");
                    let response = client.get(primary_key_path).dispatch();
                    assert_eq!(response.status(), Status::Ok, "get");

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
async fn index_protected(
    _user: AuthenticatedUser<auth::Forward>,
    config: &State<Config>,
) -> Result<Template, Error> {
    self::frontend::render_dashboard(&config.database())
}

#[get("/", rank = 2)]
async fn index_public() -> Template {
    Template::render("login", context! {})
}

#[get("/<file..>", rank = 10)]
async fn serve_files(file: PathBuf, config: &State<Config>) -> Option<NamedFile> {
    config.send_asset(file).await.ok()
}

create_routes!(crate::backend::person::Person {
    module: person,
    add_json: "/persons",
    add_frontend: "/persons/new",
    get_single: "/persons/<id>",
    get_multiple: "/persons?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::person::Group {
    module: group,
    add_json: "/groups",
    add_frontend: "/groups/new",
    get_single: "/groups/<id>",
    get_multiple: "/groups?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::document::Document {
    module: document,
    add_json: "/documents",
    add_frontend: "/documents/new",
    get_single: "/documents/<id>",
    get_multiple: "/documents?<limit>&<offset>&<order>"
});

#[get("/documents/<id>/pdf")]
async fn download_document(
    id: i64,
    state: &State<Config>,
    _user: AuthenticatedUser,
) -> Result<PdfOutput, Error> {
    PdfOutput::new(&state.database(), PrimaryKey::from(id))
}

create_routes!(crate::backend::user::User {
    module: user,
    add_json: "/users",
    add_frontend: "/users/new",
    get_single: "/users/<id>",
    get_multiple: "/users?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::accounting::Account {
    module: account,
    add_json: "/accounts",
    add_frontend: "/accounts/new",
    get_single: "/accounts/<id>",
    get_multiple: "/accounts?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::accounting::Category {
    module: category,
    add_json: "/categories",
    add_frontend: "/categories/new",
    get_single: "/categories/<id>",
    get_multiple: "/categories?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::accounting::CostCenter {
    module: cost_center,
    add_json: "/cost_centers",
    add_frontend: "/cost_centers/new",
    get_single: "/cost_centers/<id>",
    get_multiple: "/cost_centers?<limit>&<offset>&<order>"
});

create_routes!(crate::backend::accounting::Entry {
    module: entry,
    add_json: "/entries",
    add_frontend: "/entries/new",
    get_single: "/entries/<id>",
    get_multiple: "/entries?<limit>&<offset>&<order>"
});

/// Read a value from STDIN and return it without whitespace.
fn read_value(message: &'static str) -> String {
    let mut input = String::new();
    loop {
        println!("{}", message);
        match std::io::stdin().read_line(&mut input) {
            Ok(_) if input.trim().len() > 1 => {
                // Remove any whitespace in the begin
                input.truncate(input.trim_end().len());
                break input;
            }
            _ => {
                input.clear();
            }
        }
    }
}

/// Load the database, insert a default user if not specified, or kill the application on failure.
fn load_database() -> Database {
    let command_line_args: Vec<String> = std::env::args().collect();
    let (new_user, database) = match &command_line_args.as_slice() {
        &[_, path] => {
            let path = std::path::Path::new(&path);
            let database = match Database::open(path) {
                Ok(database) => database,
                Err(db_error) => {
                    eprintln!("Creating the database failed: {}", db_error);
                    std::process::exit(-1)
                }
            };

            // Prepare the first user, if not specified
            match backend::user::User::select_all(&database) {
                Ok(users) if users.len() == 0 => {
                    let user_name = read_value("Please enter the first user name: ");
                    let password = read_value("Please enter the password: ");
                    (Some((user_name, password)), database)
                }
                Err(error) => {
                    eprintln!("Creating the database failed: {}", error);
                    std::process::exit(-1)
                }
                _ => (None, database),
            }
        }
        _ => {
            // Create the database in memory and prepare the default user
            let database = Database::in_memory().expect("valid database");
            (
                Some((String::from("admin"), String::from("test1234"))),
                database,
            )
        }
    };

    if let Some((username, password)) = new_user {
        let mut admin = crate::backend::user::User::create_default(&database);
        admin.password_hash = crate::backend::user::PasswordHash::new(&username, &password);
        admin.username = username;
        admin.insert(&database).expect("unable to add Admin user");
    }

    database
}

#[launch]
fn rocket() -> _ {
    let database = load_database();
    let config = match Config::from_env(database) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1)
        }
    };

    rocket::build()
        .manage(config)
        .attach(Template::fairing())
        .register("/", catchers![error_handler])
        .mount(
            "/",
            write_routes!(
                person,
                group,
                document,
                user,
                category,
                cost_center,
                entry,
                account
                    + (
                        index_protected,
                        index_public,
                        serve_files,
                        login,
                        login_html,
                        logout,
                        download_document
                    )
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::{auth, rocket, Config};
    use crate::backend::database::{DefaultGenerator, Insertable};
    use rocket::{http::ContentType, local::blocking::Client, State};
    use rocket_dyn_templates::context;

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
        callback: impl Fn(&crate::backend::database::Database) -> T,
    ) -> (Client, T) {
        let result = {
            let database_container: &State<Config> = State::get(&engine).expect("valid database");
            let database = database_container.database();

            let mut user = crate::backend::user::User::create_default(&database);
            user.username = String::from(&credentials.user);
            user.password_hash =
                crate::backend::user::PasswordHash::new(&credentials.user, &credentials.password);
            user.insert(&database).expect("user insertion sucessfull");

            callback(&database)
        };

        (Client::tracked(engine).expect("valid client"), result)
    }

    pub fn login_with_callback<P: rocket::Phase, T>(
        engine: rocket::Rocket<P>,
        callback: impl Fn(&crate::backend::database::Database) -> T,
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
        let recieved_login_page = client
            .get("/")
            .dispatch()
            .into_string()
            .expect("valid string");

        // Simulate login
        let creation_response = client
            .post("/users/login")
            .header(ContentType::Form)
            .body("user=Chris&password=test1234")
            .dispatch();
        assert_eq!(creation_response.status(), rocket::http::Status::SeeOther);

        // Now we get the dashboard!
        let recieved_dashboard = client
            .get("/")
            .dispatch()
            .into_string()
            .expect("valid string");

        assert_ne!(recieved_login_page, recieved_dashboard);
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
    fn test_document_pdf() {
        let engine = rocket();
        let example_data = vec![42u8, 41, 40];
        let example = {
            let state: &State<Config> = State::get(&engine).expect("valid database");

            let mut example = crate::backend::document::Document::create_default(&state.database());
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
