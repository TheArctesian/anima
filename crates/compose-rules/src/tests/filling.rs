//! The variables: filled by this program, from the environment file of the item only.

use super::{ENVIRONMENT, Rule, SOUND, checked, refusal, top, with};
use crate::{Node, Refusal, Settings, check};

fn app(key: &str) -> Node {
    let document = checked(SOUND).unwrap().document;
    document
        .get("services")
        .unwrap()
        .get("app")
        .unwrap()
        .get(key)
        .unwrap()
        .clone()
}

#[test]
fn host_028_a_variable_is_filled_from_the_environment_file_or_from_its_default() {
    assert_eq!(app("image").text(), Some("example/tracks:0.36"));
    let url = app("environment")
        .get("DATABASE_URL")
        .unwrap()
        .text()
        .map(str::to_owned);
    assert_eq!(url.as_deref(), Some("postgres://tracks:hunter2@db/tracks"));
    let tagged = check(SOUND, "DB_PASSWORD=x\nTAG=0.40\n", &Settings::default()).unwrap();
    let image = tagged
        .document
        .get("services")
        .unwrap()
        .get("app")
        .unwrap()
        .get("image");
    assert_eq!(image.and_then(Node::text), Some("example/tracks:0.40"));
}

#[test]
fn host_028_two_dollars_are_one_and_a_filled_value_is_not_filled_again() {
    let document = checked(SOUND).unwrap().document;
    let content = document
        .get("configs")
        .unwrap()
        .get("settings")
        .unwrap()
        .get("content");
    assert_eq!(content.and_then(Node::text), Some("price: $5\n"));
    // The value of a variable that holds `${...}` stays as text.
    let nested = check(SOUND, "DB_PASSWORD=${TAG}\n", &Settings::default()).unwrap();
    let app = nested.document.get("services").unwrap().get("app").unwrap();
    let url = app
        .get("environment")
        .unwrap()
        .get("DATABASE_URL")
        .and_then(Node::text);
    assert_eq!(url, Some("postgres://tracks:${TAG}@db/tracks"));
}

#[test]
fn host_028_a_name_with_no_value_and_each_other_form_of_a_dollar_are_refused() {
    for (old, new) in [
        ("${DB_PASSWORD}@db", "${MISSING}@db"),
        ("${DB_PASSWORD}@db", "$DB_PASSWORD@db"),
        ("${DB_PASSWORD}@db", "${DB_PASSWORD@db"),
        ("${DB_PASSWORD}@db", "${DB_PASSWORD:?needed}@db"),
        ("${DB_PASSWORD}@db", "${db_password}@db"),
        ("${DB_PASSWORD}@db", "${DB_PASSWORD-x}@db"),
        ("${DB_PASSWORD}@db", "${MISSING:-${DB_PASSWORD}}@db"),
        ("${DB_PASSWORD}@db", "${MISSING:-$$x}@db"),
        ("RAILS_ENV: production", "RAILS_ENV: a$"),
        ("RAILS_ENV: production", "${TAG:-KEY}: production"),
    ] {
        assert_eq!(
            refusal(&with(old, new)),
            top(Rule::VariableNotPermitted),
            "{new}"
        );
    }
}

#[test]
fn host_028_a_variable_cannot_bring_a_key_or_a_structure_into_the_file() {
    // The value is one scalar of text: it is never read again as YAML.
    let environment = "DB_PASSWORD=x\nTAG=1\\n    privileged: true\n";
    let filled = check(SOUND, environment, &Settings::default());
    // The filled image holds the text of the variable and so breaks the form of an image.
    assert_eq!(
        filled,
        Err(crate::Refusal::File {
            rule: Rule::ImageNotPermitted,
            service: Some(0)
        })
    );
    // A variable in a place of a key that is not permitted does not hide the key.
    let hidden = with("restart: unless-stopped", "privileged: ${TAG:-true}");
    assert_eq!(refusal(&hidden), super::app(Rule::KeyNotPermitted));
}

#[test]
fn host_028_the_environment_file_has_one_form_and_no_name_two_times() {
    for environment in [
        "DB_PASSWORD",
        "db_password=x",
        "DB_PASSWORD=a\nDB_PASSWORD=b",
        "A B=c",
    ] {
        assert_eq!(
            check(SOUND, environment, &Settings::default()),
            Err(top(Rule::VariableNotPermitted)),
            "{environment}"
        );
    }
    // The value is the text after the first `=`; quotes are part of it; a comment is not read.
    let quoted = check(
        SOUND,
        "# a comment\n\nDB_PASSWORD=\"a=b\"\n",
        &Settings::default(),
    )
    .unwrap();
    let db = quoted.document.get("services").unwrap().get("db").unwrap();
    let Some(Node::List(entries)) = db.get("environment") else {
        panic!("a list");
    };
    assert_eq!(entries[0].text(), Some("POSTGRES_PASSWORD=\"a=b\""));
    assert!(ENVIRONMENT.starts_with("DB_PASSWORD="));
}

#[test]
fn host_028_a_line_with_a_carriage_return_and_a_line_of_only_spaces_are_refused() {
    let settings = Settings::default();
    let refused = Refusal::File {
        rule: Rule::VariableNotPermitted,
        service: None,
    };
    for environment in [
        "DB_PASSWORD=hunter2\r\n",
        "DB_PASSWORD=hunter2\n\r\n",
        "DB_PASSWORD=hunter2\rOTHER=1\n",
        "DB_PASSWORD=hunter2\n   \n",
        "DB_PASSWORD=hunter2\n\t\n",
        " # a comment\nDB_PASSWORD=hunter2\n",
    ] {
        assert_eq!(
            check(SOUND, environment, &settings),
            Err(refused),
            "{environment:?}"
        );
    }
    for environment in [
        "DB_PASSWORD=hunter2",
        "DB_PASSWORD=hunter2\n",
        "\n# a comment\n\nDB_PASSWORD=hunter2\n\n",
        "DB_PASSWORD=hunter2\nEMPTY=\nSPACES=  a b  \n",
    ] {
        assert!(
            check(SOUND, environment, &settings).is_ok(),
            "{environment:?}"
        );
    }
}
