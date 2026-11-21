//! The fields about a database (R-216, task H24): the need of an application and the server
//! of an entry of kind `database`.

use super::{COMPOSE, codes, replaced, sound, with_entry};
use crate::{
    Code, Recipe, check,
    entry::{Engine, Need, Server, Variables},
};

const NEED: &str = r#""database": {
      "engine": "postgresql",
      "lowest_major": 16,
      "extensions": ["pg_trgm"],
      "variables": { "password": "NOTES_SECRET" }
    },
    "images""#;

const SERVER: &str = r#""kind": "database",
    "server": { "engine": "postgresql", "major": 18, "extensions": ["pg_trgm"] },
    "images""#;

/// [`sound`] as an entry of kind `database` whose server exposes 8080 and publishes nothing.
fn server() -> Recipe {
    let exposed = replaced(
        COMPOSE,
        "    ports:\n      - target: 8080\n        published: ${NOTES_PORT:-8080}\n        host_ip: 127.0.0.1\n        protocol: tcp\n",
        "    expose:\n      - \"8080\"\n",
    );
    let entry = with_entry(r#""images""#, SERVER).entry;
    let tcp = entry.map(|text| {
        replaced(
            &text,
            r#""reach": "http",
      "path": "/health",
      "statuses": [200],"#,
            r#""reach": "tcp",
      "path": null,
      "statuses": [],"#,
        )
    });
    Recipe {
        entry: tcp,
        compose: Some(exposed),
        ..sound()
    }
}

#[test]
fn h24_an_application_that_names_its_connection_variables_passes() {
    assert_eq!(check(&with_entry(r#""images""#, NEED)), []);
}

#[test]
fn h24_a_need_with_no_variable_or_a_variable_that_is_not_generated_is_refused() {
    let none = NEED.replace(r#"{ "password": "NOTES_SECRET" }"#, "{}");
    let not_generated = NEED.replace("NOTES_SECRET", "NOTES_PASSWORD");
    let twice = NEED.replace(
        r#"{ "password": "NOTES_SECRET" }"#,
        r#"{ "user": "NOTES_SECRET", "password": "NOTES_SECRET" }"#,
    );
    let no_major = NEED.replace(r#""lowest_major": 16"#, r#""lowest_major": 0"#);
    for need in [none, not_generated, twice, no_major] {
        assert_eq!(
            codes(&with_entry(r#""images""#, &need)),
            [Code::DatabaseNotDescribed],
            "{need}"
        );
    }
}

#[test]
fn h24_a_database_server_publishes_no_port_and_its_test_reaches_an_exposed_port() {
    assert_eq!(check(&server()), []);
    let published = Recipe {
        compose: sound().compose,
        ..server()
    };
    assert_eq!(
        codes(&published),
        [Code::DatabasePortPublished, Code::TestNotComplete]
    );
}

#[test]
fn h24_a_database_entry_describes_its_server_and_needs_no_database() {
    let no_server = with_entry(r#""images""#, r#""kind": "database", "images""#);
    assert!(codes(&no_server).contains(&Code::DatabaseNotDescribed));
    let server_of_an_application =
        with_entry(r#""images""#, &SERVER.replace(r#""kind": "database","#, ""));
    assert!(codes(&server_of_an_application).contains(&Code::DatabaseNotDescribed));
}

#[test]
fn h24_a_server_meets_a_need_with_its_engine_major_and_extensions() {
    let server = Server {
        engine: Engine::Postgresql,
        major: 18,
        extensions: vec!["pg_trgm".to_owned()],
    };
    let need = |lowest_major, extension: &str| Need {
        engine: Engine::Postgresql,
        lowest_major,
        extensions: vec![extension.to_owned()],
        variables: Variables::default(),
    };
    assert!(need(16, "pg_trgm").met_by(&server));
    assert!(need(18, "pg_trgm").met_by(&server));
    assert!(!need(19, "pg_trgm").met_by(&server));
    assert!(!need(16, "postgis").met_by(&server));
}
