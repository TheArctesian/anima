use async_graphql::Value;

use super::*;
use crate::devices::graphql_error::to_graphql;

fn code_of(error: &async_graphql::Error) -> Value {
    error
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("code"))
        .cloned()
        .expect("every devices error has a code")
}

#[test]
fn k7_not_found_has_code_not_found_and_names_what_is_missing() {
    let error = to_graphql(&Error::NotFound(Missing::Site(home())));
    assert_eq!(code_of(&error), Value::from("NOT_FOUND"));
    assert!(error.message.contains(HOME));
}

#[test]
fn k7_validation_lists_each_field_with_its_message() {
    let error = to_graphql(&Error::Validation(vec![
        FieldError::new("first", "Too big."),
        FieldError::new("siteId", "Not a site."),
    ]));
    assert_eq!(code_of(&error), Value::from("VALIDATION"));
    let fields = error
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("fields"))
        .cloned()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(
        fields,
        serde_json::json!([
            { "field": "first", "message": "Too big." },
            { "field": "siteId", "message": "Not a site." },
        ])
    );
}

#[test]
fn k7_database_failure_is_internal_with_a_generic_message() {
    let error = to_graphql(&Error::Database(sqlx::Error::Protocol(
        "SELECT secret FROM devices.devices".into(),
    )));
    assert_eq!(code_of(&error), Value::from("INTERNAL"));
    assert!(!error.message.contains("SELECT") && !error.message.contains("devices."));
}
