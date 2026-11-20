//! `Subscription.reminderFirings`: an open application gets each firing when it is sent, and the
//! hub counts the open subscriptions (K-E6 Section 5; REM-008).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    engine::reminders::{Live, Sending, send_waiting},
    fleet::{
        access::Identity,
        events::{Actor, Bus, Operator},
    },
    ids::OperatorId,
    schema::{Dependencies, build_schema},
};
use async_graphql::{Request, futures_util::StreamExt as _};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{FIRING, daily};
use crate::hub::{Hub, dependencies, set_operator, start, test_vault};

fn fixed_clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(start()))
}

fn signed_in(request: Request) -> Request {
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    Identity::for_tests(operator, start() + TimeDelta::days(3650)).attach(request)
}

fn parts(pool: &PgPool, live: &Live) -> Dependencies {
    Dependencies {
        reminders: live.clone(),
        ..dependencies(pool, fixed_clock(), Bus::new(16))
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_reminder_firings_gives_a_firing_when_it_is_sent_and_counts_the_subscription(
    pool: PgPool,
) {
    set_operator(&pool).await;
    let live = Live::default();
    // The mutations go through the HTTP edge with a token; the subscription is read from a
    // schema that has the same `Live`.
    let hub = Hub::serving(&pool, parts(&pool, &live));
    let token = hub.token().await;
    let schema = build_schema(parts(&pool, &live));
    let made = hub
        .post(
            Some(&token),
            "mutation($i: ReminderRuleInput!) { createReminderRule(input: $i) { id } }",
            json!({ "i": daily() }),
        )
        .await;
    let rule = made["data"]["createReminderRule"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let query = format!("subscription {{ reminderFirings {{ {FIRING} sentAt }} }}");
    let mut stream = schema.execute_stream(signed_in(Request::new(query)));
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "nothing was sent yet");
    assert_eq!(live.open(), 1);

    let tested = hub
        .post(
            Some(&token),
            "mutation($id: ID!) { testReminderRule(id: $id) { id } }",
            json!({ "id": rule }),
        )
        .await;
    let sending = Sending {
        db: pool.clone(),
        bus: Bus::new(16),
        vault: test_vault(),
        live: live.clone(),
    };
    let now = start() + TimeDelta::minutes(1);
    assert_eq!(send_waiting(&sending, now).await.unwrap(), 1);

    let next = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap();
    assert!(next.errors.is_empty(), "{:?}", next.errors);
    let firing = next.data.into_json().unwrap()["reminderFirings"].clone();
    assert_eq!(firing["id"], tested["data"]["testReminderRule"]["id"]);
    assert_eq!(
        (&firing["state"], &firing["text"], &firing["ruleId"]),
        (&json!("SENT"), &json!("Stand up and stretch"), &json!(rule))
    );
    assert_eq!(firing["sentAt"], "2030-05-05T12:01:00+00:00");
    let queued: i64 = sqlx::query_scalar("select count(*) from notifications.outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, 0, "the application shows it");

    drop(stream);
    assert_eq!(live.open(), 0, "a closed subscription is not counted");
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_reminder_firings_refuses_a_request_with_no_sign_in(pool: PgPool) {
    let live = Live::default();
    let schema = build_schema(parts(&pool, &live));
    let mut stream = schema.execute_stream(Request::new("subscription { reminderFirings { id } }"));
    let refused = stream.next().await.unwrap();
    assert_eq!(refused.errors.len(), 1, "{:?}", refused.errors);
    assert_eq!(live.open(), 0, "it is not an open application");
}
