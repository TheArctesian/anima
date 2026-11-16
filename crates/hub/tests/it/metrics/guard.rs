//! `capabilities` answers a signed-in caller only, even without the hub-wide sign-in check
//! (API-003, CAP-030): the schema here holds the metrics queries alone.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{access::Identity, events::Operator, metrics::graphql::Query},
    ids::OperatorId,
};
use async_graphql::{EmptyMutation, EmptySubscription, Request, Schema, value};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
const LIST: &str = "{ capabilities { os } }";

fn now() -> DateTime<Utc> {
    "2026-11-15T09:00:00Z".parse().unwrap()
}

fn signed_in(query: &str) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
        actor: anima_hub::fleet::events::Actor::for_tests(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(3650)).attach(Request::new(query))
}

#[sqlx::test(migrations = "./migrations")]
async fn cap_030_capabilities_refuses_a_caller_who_is_not_signed_in(pool: PgPool) {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(pool)
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .finish();
    let refused = schema.execute(LIST).await;
    let [error] = refused.errors.as_slice() else {
        panic!("expected one error, got {:?}", refused.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("UNAUTHENTICATED")), "{error:?}");
    assert_eq!(refused.data, value!(null));

    let answered = schema.execute(signed_in(LIST)).await;
    assert!(answered.errors.is_empty(), "{:?}", answered.errors);
    assert_eq!(answered.data, value!({ "capabilities": [] }));
}
