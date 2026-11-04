//! `ConnectionTestInput` as the operator sends it (DEV-013): which subject it names, and K1's
//! defaults.

use async_graphql::{InputType, value};

use crate::sessions::{
    graphql::connection_test::ConnectionTestInput,
    trust::{TestSubject, Unsaved},
};

fn subject(input: async_graphql::Value) -> TestSubject {
    let input = ConnectionTestInput::parse(Some(input)).unwrap_or_else(|_| panic!("parses"));
    input
        .into_subject()
        .unwrap_or_else(|_| panic!("names a subject"))
}

#[test]
fn dev_013_a_host_alone_is_tested_on_ssh_port_22() {
    let TestSubject::Unsaved(unsaved) = subject(value!({ "host": " 10.10.0.2 " })) else {
        panic!("unsaved values");
    };
    let expected = Unsaved {
        host: "10.10.0.2".to_owned(),
        port: 22,
    };
    assert_eq!(unsaved, expected);
}

#[test]
fn dev_013_the_form_port_replaces_the_default() {
    let ssh = value!({ "enabled": true, "port": 2222, "username": "operator", "auth": "PASSWORD" });
    let TestSubject::Unsaved(unsaved) = subject(value!({ "host": "pve-01.lan", "ssh": ssh }))
    else {
        panic!("unsaved values");
    };
    assert_eq!(unsaved.port, 2222);
}
