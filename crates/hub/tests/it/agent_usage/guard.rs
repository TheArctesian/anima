//! `agentUsage` answers a signed-in caller only, even without the hub-wide sign-in check
//! (API-003, AIS-020): the schema here holds the agent usage query alone.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{access::Identity, agent_usage::graphql::Query, events::Operator},
    ids::OperatorId,
};
use async_graphql::{EmptyMutation, EmptySubscription, Request, Schema, value};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
const SUMS: &str = "{ agentUsage { firstDay lastDay total { sessions estimatedCostUsd } } }";

fn now() -> DateTime<Utc> {
    "2026-11-17T09:00:00Z".parse().unwrap()
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
async fn ais_020_agent_usage_refuses_a_caller_who_is_not_signed_in(pool: PgPool) {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(pool)
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .finish();
    let refused = schema.execute(SUMS).await;
    let [error] = refused.errors.as_slice() else {
        panic!("expected one error, got {:?}", refused.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("UNAUTHENTICATED")), "{error:?}");
    assert_eq!(refused.data, value!(null));

    let answered = schema.execute(signed_in(SUMS)).await;
    assert!(answered.errors.is_empty(), "{:?}", answered.errors);
    assert_eq!(
        answered.data,
        value!({ "agentUsage": { "firstDay": "2026-10-19", "lastDay": "2026-11-17",
                                 "total": { "sessions": 0, "estimatedCostUsd": null } } })
    );
}
