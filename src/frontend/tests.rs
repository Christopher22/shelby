//! Some tests directly refering to the frontend

use std::str::FromStr;

use rocket::{
    futures::stream::repeat,
    http::{hyper::Uri, uri::Origin, Accept, MediaType, QMediaType, Status},
    serde::json,
    State,
};

use crate::{
    backend::database::{DefaultGenerator, Insertable},
    rocket,
    tests::login,
    Config,
};

#[test]
fn test_group_json_and_html() {
    use crate::backend::person::Group;

    let (client, path) = {
        let engine = rocket();
        let primary_key = {
            let state: &State<Config> = State::get(&engine).expect("valid database");
            let default_group = Group::create_default(&state.database());
            default_group
                .insert(&state.database())
                .expect("Insert failed")
        };
        let client = crate::tests::login(engine);
        (
            client,
            Origin::parse_owned(format!("/groups/{}", primary_key.0)).expect("valid origin"),
        )
    };

    // Test JSON
    {
        let mut response = client.get(path.clone());
        response.add_header(Accept::new(QMediaType(MediaType::JSON, None)));
        let response = response.dispatch();
        assert_eq!(response.status(), Status::Ok, "get json");
        let response = response.into_string().expect("valid str");
        let _: Group = json::from_str(&response).expect("valid json");
    }

    // Test JSON as default
    {
        let response = client.get(path.clone()).dispatch();
        assert_eq!(response.status(), Status::Ok, "get default");
        let response = response.into_string().expect("valid str");
        let _: Group = json::from_str(&response).expect("valid json");
    }

    // Test HTML
    {
        let mut response = client.get(path);
        response.add_header(Accept::new(QMediaType(MediaType::HTML, None)));
        let response = response.dispatch();
        assert_eq!(response.status(), Status::Ok, "get html");
        let response = response.into_string().expect("valid str");
        assert!(response.contains("<body"))
    }
}
