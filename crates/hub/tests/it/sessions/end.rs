//! `endSession` (D-10, K7) and the sweep of tickets that expired unredeemed (D-2): each
//! session ends once, with one `session.end` audit entry.

mod shutdown;

use anima_hub::fleet::sessions::Presented;
use std::time::Duration;

use anima_hub::fleet::{
    events::ClientAddr,
    sessions::{Actor, EndCause, EndReason, Live},
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use super::{
    DOCKER_HOST, OLLAMA, PVE_01,
    harness::{TestSchema, error_json, operator, schema_at},
    now,
    starting::{
        client, end_audits, end_request, event_count, ready, redeem, started, started_on_pve_01,
        stored,
    },
};

async fn end(schema: &TestSchema, id: &str) -> Value {
    let response = schema.execute(end_request(id)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["endSession"].clone()
}

fn instant(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_session_on_a_pending_ticket_ends_the_row_operator_once(pool: PgPool) {
    let live = Live::default();
    let ticket = started_on_pve_01(&pool, &schema_at(&pool, now(), &live)).await;
    let id = ticket["sessionId"].as_str().unwrap();
    let later = schema_at(&pool, now() + TimeDelta::seconds(5), &live);

    let ended = end(&later, id).await;
    assert_eq!(ended["id"], id);
    assert_eq!(ended["endReason"], "OPERATOR");
    assert_eq!(instant(&ended["endedAt"]), now() + TimeDelta::seconds(5));
    assert_eq!(
        stored(&pool, id).await.end_reason.as_deref(),
        Some("operator")
    );
    assert!(
        redeem(&live, &ticket, now()).is_none(),
        "the ticket is withdrawn"
    );

    // Ending it again returns the row unchanged and writes nothing.
    let again = end(&schema_at(&pool, now() + TimeDelta::minutes(1), &live), id).await;
    assert_eq!(again, ended);
    let audits = end_audits(&pool, id).await;
    assert_eq!(audits.len(), 1);
    assert_eq!(
        audits[0].change,
        serde_json::json!({ "reason": "operator" })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_session_tells_the_open_tunnel_and_returns_the_row_it_ended(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let ticket = started_on_pve_01(&pool, &schema).await;
    let id = ticket["sessionId"].as_str().unwrap().to_owned();
    let mut tunnel = redeem(&live, &ticket, now()).unwrap().tunnel;

    // A stand-in for the tunnel task: on the signal it ends its row, then drops its handle.
    let tunnel_pool = pool.clone();
    let tunnel_id = id.clone();
    let task = tokio::spawn(async move {
        let cause = tunnel.ended().await;
        sqlx::query(
            "update sessions.sessions set ended_at = $2, end_reason = 'operator' \
             where id = $1::text::uuid",
        )
        .bind(&tunnel_id)
        .bind(now() + TimeDelta::seconds(2))
        .execute(&tunnel_pool)
        .await
        .unwrap();
        drop(tunnel);
        cause
    });

    let ended = end(&schema, &id).await;
    let who = Actor {
        operator: operator(),
        client: client(),
        at: now(),
    };
    let cause = EndCause {
        reason: EndReason::Operator,
        by: Some(who),
    };
    assert_eq!(
        task.await.unwrap(),
        cause,
        "the tunnel learns who ended it (LOG-005)"
    );
    assert_eq!(ended["endReason"], "OPERATOR");
    assert_eq!(instant(&ended["endedAt"]), now() + TimeDelta::seconds(2));
    assert!(
        end_audits(&pool, &id).await.is_empty(),
        "the tunnel writes its own end entry (T5)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k7_end_session_on_an_unknown_or_malformed_id_is_refused(pool: PgPool) {
    let schema = schema_at(&pool, now(), &Live::default());
    let unknown = schema
        .execute(end_request("0190a000-0000-7000-8010-0000000000ff"))
        .await;
    assert_eq!(error_json(&unknown)["extensions"]["code"], "NOT_FOUND");
    let malformed = schema.execute(end_request("session-1")).await;
    let error = error_json(&malformed);
    assert_eq!(error["extensions"]["code"], "VALIDATION");
    assert_eq!(error["extensions"]["fields"][0]["field"], "id");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_009_the_sweep_ends_expired_unredeemed_tickets_timeout_once(pool: PgPool) {
    let live = Live::default();
    ready(&pool, &[PVE_01, DOCKER_HOST, OLLAMA]).await;
    let first = schema_at(&pool, now(), &live);
    let expired = started(&first, PVE_01).await;
    let opened = started(&first, OLLAMA).await;
    let _tunnel = redeem(&live, &opened, now() + TimeDelta::seconds(1)).unwrap();
    let spent = started(&first, OLLAMA).await;
    let elsewhere = ClientAddr("10.10.0.99".parse().unwrap());
    let token = spent["token"].as_str().unwrap();
    assert!(
        live.redeem(Presented::terminal(token, elsewhere), now())
            .is_err()
    );
    assert!(
        redeem(&live, &spent, now()).is_none(),
        "spent by the refusal"
    );
    let expired_id = expired["sessionId"].as_str().unwrap();
    let opened_id = opened["sessionId"].as_str().unwrap();

    // One second before expiry nothing is swept; at the expiry instant it is.
    started(
        &schema_at(&pool, now() + TimeDelta::seconds(29), &live),
        DOCKER_HOST,
    )
    .await;
    assert_eq!(stored(&pool, expired_id).await.end_reason, None);
    started(
        &schema_at(&pool, now() + TimeDelta::seconds(30), &live),
        DOCKER_HOST,
    )
    .await;
    let row = stored(&pool, expired_id).await;
    assert_eq!(row.end_reason.as_deref(), Some("timeout"));
    assert_eq!(row.ended_at, Some(now() + TimeDelta::seconds(30)));
    let spent_row = stored(&pool, spent["sessionId"].as_str().unwrap()).await;
    assert_eq!(spent_row.end_reason.as_deref(), Some("timeout"));

    // Later sweeps leave it alone, and never touch the redeemed one.
    started(
        &schema_at(&pool, now() + TimeDelta::hours(1), &live),
        DOCKER_HOST,
    )
    .await;
    let audits = end_audits(&pool, expired_id).await;
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].change, serde_json::json!({ "reason": "timeout" }));
    assert_eq!(stored(&pool, opened_id).await.end_reason, None);
    assert!(redeem(&live, &expired, now()).is_none());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_session_returns_the_live_row_when_the_tunnel_does_not_finish(pool: PgPool) {
    let live = Live::with_tunnel_end_wait(Duration::from_millis(50));
    let schema = schema_at(&pool, now(), &live);
    let ticket = started_on_pve_01(&pool, &schema).await;
    let id = ticket["sessionId"].as_str().unwrap();
    let stuck = redeem(&live, &ticket, now()).unwrap().tunnel;

    let row = end(&schema, id).await;
    assert_eq!(
        row["endReason"],
        Value::Null,
        "still live: the tunnel never dropped"
    );
    assert_eq!(row["endedAt"], Value::Null);
    assert_eq!(stored(&pool, id).await.end_reason, None);
    drop(stuck);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_session_on_a_ticket_spent_by_a_refusal_ends_it_once(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let ticket = started_on_pve_01(&pool, &schema).await;
    let id = ticket["sessionId"].as_str().unwrap();
    let elsewhere = ClientAddr("10.10.0.99".parse().unwrap());
    assert!(
        live.redeem(
            Presented::terminal(ticket["token"].as_str().unwrap(), elsewhere),
            now()
        )
        .is_err()
    );

    let ended = end(&schema, id).await;
    assert_eq!(ended["endReason"], "OPERATOR");
    // The sweep no longer holds it, so a later start ends nothing more.
    started(
        &schema_at(&pool, now() + TimeDelta::hours(1), &live),
        PVE_01,
    )
    .await;
    assert_eq!(end_audits(&pool, id).await.len(), 1);
    assert_eq!(
        stored(&pool, id).await.end_reason.as_deref(),
        Some("operator")
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_009_a_late_sweep_ends_the_row_at_the_tickets_expiry_not_at_now(pool: PgPool) {
    let live = Live::default();
    ready(&pool, &[PVE_01, DOCKER_HOST]).await;
    let ticket = started(&schema_at(&pool, now(), &live), PVE_01).await;
    started(
        &schema_at(&pool, now() + TimeDelta::seconds(45), &live),
        DOCKER_HOST,
    )
    .await;
    let row = stored(&pool, ticket["sessionId"].as_str().unwrap()).await;
    assert_eq!(row.end_reason.as_deref(), Some("timeout"));
    assert_eq!(row.ended_at, Some(now() + TimeDelta::seconds(30)));
}

/// Ends that never open a tunnel write the `session.end` audit entry and, by decision, no
/// event (K4 §5 describes tunnel ends).
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_004_ends_without_a_tunnel_write_audit_and_no_event(pool: PgPool) {
    let live = Live::default();
    ready(&pool, &[PVE_01, DOCKER_HOST]).await;
    let first = schema_at(&pool, now(), &live);
    let withdrawn = started(&first, PVE_01).await;
    let expired = started(&first, PVE_01).await;
    let spent = started(&first, PVE_01).await;
    let elsewhere = ClientAddr("10.10.0.99".parse().unwrap());
    assert!(
        live.redeem(
            Presented::terminal(spent["token"].as_str().unwrap(), elsewhere),
            now()
        )
        .is_err()
    );
    end(&first, withdrawn["sessionId"].as_str().unwrap()).await;
    started(
        &schema_at(&pool, now() + TimeDelta::minutes(1), &live),
        DOCKER_HOST,
    )
    .await;

    for ticket in [&withdrawn, &expired, &spent] {
        assert_eq!(
            end_audits(&pool, ticket["sessionId"].as_str().unwrap())
                .await
                .len(),
            1
        );
    }
    assert_eq!(event_count(&pool).await, 0);
}
