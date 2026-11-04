//! Syntax and validation errors keep their position and code but never quote the request
//! (H-6 (e) and (f), LOG-007).

use async_graphql::{
    EmptyMutation, EmptySubscription, Enum, InputObject, Object, Request, Response, Schema,
    Variables,
};
use serde_json::{Value, json};

use super::{
    capture::capture_log,
    leaks::{SECRET, hardened},
};
use crate::schema::harden;

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum Shade {
    Dark,
}

#[derive(InputObject)]
struct Filter {
    name: String,
    shade: Option<Shade>,
}

struct Query;

#[Object]
impl Query {
    async fn find(&self, filter: Filter) -> bool {
        filter.name.is_empty() && filter.shade == Some(Shade::Dark)
    }
}

fn shaded() -> Schema<Query, EmptyMutation, EmptySubscription> {
    harden(Schema::build(Query, EmptyMutation, EmptySubscription), true).finish()
}

fn errors(response: &Response) -> Vec<Value> {
    response
        .errors
        .iter()
        .map(|error| serde_json::to_value(error).unwrap())
        .collect()
}

#[tokio::test]
async fn h_6_e_a_syntax_error_keeps_its_position_and_not_the_query_line() {
    let (capture, _guard) = capture_log("trace");
    // The selection set is never closed.
    let query = format!(r#"mutation {{ login(input: {{ username: "a", password: "{SECRET}" }}) "#);

    let response = hardened().execute(query.as_str()).await;

    let [error] = errors(&response).try_into().expect("one error");
    assert_eq!(error["message"], "The request is not valid GraphQL.");
    assert!(
        error["locations"][0]["column"].as_u64().unwrap() > 1,
        "{error}"
    );
    assert!(error.get("extensions").is_none(), "{error}");
    assert!(!error.to_string().contains(SECRET), "{error}");
    assert!(!capture.text().contains(SECRET));
    // Without the boundary, async-graphql quotes the line: this is what the test guards.
    let plain = Schema::new(super::leaks::Query, EmptyMutation, EmptySubscription);
    assert!(
        plain.execute(query.as_str()).await.errors[0]
            .message
            .contains(SECRET)
    );
}

/// Each request names a value or input field async-graphql's validation would quote, with the
/// argument path the error may keep.
fn quoting() -> [(Request, &'static str); 4] {
    let variables = |filter| Variables::from_json(json!({ "f": filter }));
    [
        (
            Request::new(format!(
                r#"{{ find(filter: {{ name: "a", {}: 1 }}) }}"#,
                SECRET.replace('-', "_")
            )),
            "filter",
        ),
        (
            Request::new("query($f: Filter!) { find(filter: $f) }")
                .variables(variables(json!({ "name": "a", SECRET: 1 }))),
            "filter",
        ),
        (
            Request::new("query($f: Filter!) { find(filter: $f) }")
                .variables(variables(json!({ "name": "a", "shade": SECRET }))),
            "filter.shade",
        ),
        (
            Request::new(format!(
                "query($f: Filter = {{ name: \"a\", shade: {} }}) {{ find(filter: $f) }}",
                SECRET.replace('-', "_")
            )),
            "",
        ),
    ]
}

#[tokio::test]
async fn h_6_f_an_unknown_input_field_or_value_is_named_by_its_path_only() {
    let (capture, _guard) = capture_log("trace");

    for (request, path) in quoting() {
        let response = shaded().execute(request).await;
        let all = errors(&response);
        let error = all
            .iter()
            .find(|error| error["extensions"]["code"] == "VALIDATION")
            .unwrap_or_else(|| panic!("a VALIDATION error in {all:?}"));
        assert_eq!(error["message"], "An argument has the wrong type or shape.");
        assert!(error["locations"].is_array(), "{error}");
        if !path.is_empty() {
            assert_eq!(error["extensions"]["fields"][0]["field"], path, "{error}");
        }
        let text = Value::Array(all).to_string();
        assert!(!text.contains(SECRET), "{text}");
        assert!(!text.contains(&SECRET.replace('-', "_")), "{text}");
    }
    assert!(!capture.text().contains(SECRET));
}

#[tokio::test]
async fn h_6_f_other_validation_errors_pass_unchanged() {
    let response = shaded().execute("{ nosuch }").await;
    let [error] = errors(&response).try_into().expect("one error");
    assert_eq!(
        error["message"],
        r#"Unknown field "nosuch" on type "Query"."#
    );
}
