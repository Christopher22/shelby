#[macro_use]
extern crate rocket;

mod error;

use std::sync::Mutex;

use rocket::{serde::json::Json, State};
use shelby_backend::{
    document::Document,
    person::{Group, Person},
    Database, IndexableDatebaseEntry, PrimaryKey, Record,
};

type DatabaseState = State<Mutex<Database>>;
pub use self::error::Error;

macro_rules! create_routes {
    ($path: literal => $database_entry: ident ($function_name: ident)) => {
        paste::paste! {
            #[post($path, format = "json", data = "<database_entry>")]
            fn [< add_ $function_name >](
                database_entry: Json<$database_entry>,
                state: &DatabaseState,
            ) -> Result<Json<PrimaryKey<$database_entry>>, Error> {
                database_entry
                    .0
                    .insert(&state.lock().expect("database mutex"))
                    .map(Json)
                    .map_err(Error::from)
            }

            #[get($path)]
            fn [< get_all_ $function_name s >](state: &DatabaseState) -> Result<Json<Vec<Record<$database_entry>>>, Error> {
                Ok(Json($database_entry::select_all(
                    &state.lock().expect("database mutex"),
                )?))
            }

            #[get("/persons/<id>")]
            fn [< get_ $function_name _by_id>](id: i64, state: &DatabaseState) -> Result<Json<Record<$database_entry>>, Error> {
                match $database_entry::try_select(
                    &state.lock().expect("database mutex"),
                    id
                )? {
                    Some(value) => Ok(Json(value)),
                    None => Err(Error::NotFound)
                }
            }
        }
    }
}

macro_rules! write_routes {
    ($($function_name: ident),* + $($additional: ident),*) => { paste::paste! {
        routes![$($additional),*, $(
            [< add_ $function_name >], [< get_all_ $function_name s >], [< get_ $function_name _by_id>]
        ),*]
    }};
}

#[get("/")]
fn index() -> &'static str {
    "shelby 0.1"
}

create_routes!("/persons" => Person (person));
create_routes!("/groups" => Group (group));
create_routes!("/documents" => Document (document));

#[launch]
fn rocket() -> _ {
    let database = Database::in_memory().expect("Valid database");
    rocket::build()
        .manage(Mutex::new(database))
        .mount("/", write_routes!(person, group, document + index))
}
