#[macro_use]
extern crate rocket;

mod error;
use std::sync::Mutex;

use rocket::{serde::json::Json, State, response::status};
use shelby_backend::{
    document::Document,
    person::{Group, Person},
    Database, IndexableDatebaseEntry, Record,
};

type DatabaseState = State<Mutex<Database>>;
pub use self::error::Error;

macro_rules! create_routes {
    ($path: literal ($path_id: literal) => $database_entry: ident ($function_name: ident)) => {
        paste::paste! {
            #[post($path, format = "json", data = "<database_entry>")]
            fn [< add_ $function_name >](
                database_entry: Json<$database_entry>,
                state: &DatabaseState,
            ) -> Result<status::Created<String>, Error> {
                database_entry
                    .0
                    .insert(&state.lock().expect("database mutex"))
                    .map(|primary_key| status::Created::new(primary_key.to_string()))
                    .map_err(Error::from)
            }

            #[get($path)]
            fn [< get_all_ $function_name s >](state: &DatabaseState) -> Result<Json<Vec<Record<$database_entry>>>, Error> {
                Ok(Json($database_entry::select_all(
                    &state.lock().expect("database mutex"),
                )?))
            }

            #[get($path_id)]
            fn [< get_ $function_name _by_id>](id: i64, state: &DatabaseState) -> Result<Json<Record<$database_entry>>, Error> {
                match $database_entry::try_select(
                    &state.lock().expect("database mutex"),
                    id
                )? {
                    Some(value) => Ok(Json(value)),
                    None => Err(Error::NotFound)
                }
            }

            #[cfg(test)]
            mod [< test_ $function_name >] {
                use super::{rocket, DatabaseState};
                use rocket::{http::Status, local::blocking::Client, serde::json, State};
                use shelby_backend::{DefaultGenerator, Record};

                type TargetEntity = super::$database_entry;
                const ACCESS_POINT: &'static str = $path;
                
                #[test]
                fn test_get_empty() {
                    let client = Client::tracked(rocket()).expect("valid client");
                    let response = client.get(ACCESS_POINT).dispatch();
                    assert_eq!(response.status(), Status::Ok);

                    let response_json: Vec<Record<TargetEntity>> = response.into_json().expect("valid json");
                    assert_eq!(response_json.len(), 0);
                }

                #[test]
                fn test_get() {
                    let engine = rocket();
                    let example = {
                        let database: &DatabaseState = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&database.lock().expect("database mutex"))
                    };

                    let client = Client::tracked(engine).expect("valid client");
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
                        let database: &DatabaseState = State::get(&engine).expect("valid database");
                        TargetEntity::create_default(&database.lock().expect("database mutex"))
                    };

                    let client = Client::tracked(engine).expect("valid client");
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
                    let engine = rocket();
                    let client = Client::tracked(engine).expect("valid client");
                    
                    let response = client.get(format!("{}/42", ACCESS_POINT)).dispatch();
                    assert_eq!(response.status(), Status::NotFound);
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

#[get("/")]
fn index() -> &'static str {
    "shelby 0.1"
}

create_routes!("/persons" ("/persons/<id>") => Person (person));
create_routes!("/groups" ("/groups/<id>") => Group (group));
create_routes!("/documents" ("/documents/<id>") => Document (document));

#[launch]
fn rocket() -> _ {
    let database = Database::in_memory().expect("Valid database");
    rocket::build().manage(Mutex::new(database)).mount(
        "/",
        write_routes!(person, group, document + (index)),
    )
}
