//! The RDP and VNC credential argument holds its value as a secret and never quotes it
//! (DEV-011, SEC-005, LOG-007).

use anima_hub::devices::graphql::PasswordCredential;
use async_graphql::{InputType, Name, Pos, Value, indexmap::IndexMap};

const SECRET: &str = "s3cr3t-pw-do-not-log";

fn object(fields: &[(&str, Value)]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(name, value)| (Name::new(name), value.clone()))
            .collect::<IndexMap<_, _>>(),
    )
}

fn parse(value: Value) -> Result<PasswordCredential, async_graphql::ServerError> {
    PasswordCredential::parse(Some(value)).map_err(|e| e.into_server_error(Pos::default()))
}

#[test]
fn log_007_unreadable_argument_quotes_nothing() {
    let secret = Value::from(SECRET);
    let unreadable = [
        secret.clone(),
        Value::List(vec![secret.clone()]),
        object(&[("password", Value::List(vec![secret.clone()]))]),
        object(&[("passwd", secret.clone())]),
        Value::Null,
    ];
    // An unknown extra field is refused by query validation before any parse, so it is not here.
    for value in unreadable {
        let Err(error) = parse(value) else {
            panic!("an unreadable credential must not parse");
        };
        let shown = format!("{error:?} | {}", serde_json::to_string(&error).unwrap());
        assert!(!shown.contains(SECRET), "{shown}");
        let extensions = serde_json::to_value(error.extensions.unwrap()).unwrap();
        assert_eq!(
            extensions,
            serde_json::json!({
                "code": "VALIDATION",
                "fields": [{ "field": "credential", "message": "Send a password." }],
            })
        );
    }
}

#[test]
fn sec_005_parsed_credential_shows_no_value() {
    let parsed = parse(object(&[("password", Value::from(SECRET))])).unwrap();
    assert_eq!(parsed.to_value(), Value::Null);
    let secret = parsed.into_checked().unwrap();
    assert_eq!(secret.expose(), SECRET.as_bytes());
}

#[test]
fn log_007_blank_or_oversized_password_is_refused_without_its_value() {
    let oversized = "x".repeat(1025);
    for text in ["", "   ", "\t\n", oversized.as_str()] {
        let parsed = parse(object(&[("password", Value::from(text))])).unwrap();
        let Err(errors) = parsed.into_checked() else {
            panic!("a blank or oversized password must be refused");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "credential.password");
        let message = &errors[0].message;
        assert!(
            !message.contains("1025") && !message.contains("xxx"),
            "{message}"
        );
    }
}

#[test]
fn password_of_1024_bytes_is_accepted() {
    let longest = "y".repeat(1024);
    let parsed = parse(object(&[("password", Value::from(longest.as_str()))])).unwrap();
    assert_eq!(parsed.into_checked().unwrap().expose(), longest.as_bytes());
}
