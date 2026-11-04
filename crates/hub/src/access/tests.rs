use async_graphql::{EmptyMutation, EmptySubscription, Object, Request, Schema, Value, value};

use super::{Authenticated, Operator};

struct Query;

#[Object]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn me(&self, ctx: &async_graphql::Context<'_>) -> String {
        ctx.data_unchecked::<Operator>().name.clone()
    }
}

fn schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::new(Query, EmptyMutation, EmptySubscription)
}

#[tokio::test]
async fn api_003_guard_refuses_a_request_without_an_operator() {
    let response = schema().execute("{ me }").await;

    assert_eq!(response.data, Value::Null);
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    assert_eq!(error.message, "Sign in to continue.");
    let code = error
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("code"));
    assert_eq!(code, Some(&value!("UNAUTHENTICATED")));
}

#[tokio::test]
async fn api_003_guard_passes_a_request_with_an_operator() {
    let operator = Operator {
        id: "0192f5a0-7c3e-7a52-9d1b-000000000001".parse().unwrap(),
        name: "operator".to_owned(),
    };

    let response = schema()
        .execute(Request::new("{ me }").data(operator))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data, value!({ "me": "operator" }));
}
