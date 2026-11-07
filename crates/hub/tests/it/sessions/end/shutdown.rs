//! Ends at the hub's edges (D-10, LOG-005, MV-003): `shut_down` ends every session this hub
//! holds, and `close_stale` at the next start ends what a previous process left open, each
//! with one `session.end` audit entry.

use anima_hub::sessions::Presented;
use std::time::Duration;

use anima_hub::{
    events::ClientAddr,
    sessions::{self, EndReason, Live, Tunnel},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::super::{
    DOCKER_HOST, OLLAMA, OPERATOR_ID, PVE_01, Row,
    harness::{CLIENT_ADDR, schema_at},
    insert_all, now, session_id,
    starting::{
        display, end_audits, event_count, ready, redeem, session_audits, start_request, started,
        stored,
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_008_close_stale_ends_only_open_sessions_with_hub_shutdown(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::live(2, DOCKER_HOST, 1),
            Row::ended(3, PVE_01, 2, "operator"),
        ],
    )
    .await;
    let at = now() + TimeDelta::hours(1);

    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 2);

    let rows: Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>, String)> =
        sqlx::query_as("select id, ended_at, end_reason from sessions.sessions order by id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            (session_id(1), at, "hub_shutdown".to_owned()),
            (session_id(2), at, "hub_shutdown".to_owned()),
            (
                session_id(3),
                now() + TimeDelta::minutes(7),
                "operator".to_owned()
            ),
        ]
    );
    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 0);
}

const ALICE: &str = "0190a000-0000-7000-8000-000000000002";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_005_close_stale_audits_each_row_it_ends_naming_its_operator_and_client(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::ended(2, PVE_01, 1, "operator"),
        ],
    )
    .await;
    // Another operator's session, from another address.
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at) \
         values ($1, $2::text::uuid, 'docker-host', 'ssh', $3::text::uuid, 'alice', \
                 '192.0.2.9', $4)",
    )
    .bind(session_id(3))
    .bind(DOCKER_HOST)
    .bind(ALICE)
    .bind(now())
    .execute(&pool)
    .await
    .unwrap();
    let at = now() + TimeDelta::hours(1);

    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 2);

    let audits: Vec<(String, String, String, String, String, Value)> = sqlx::query_as(
        "select target_id, operator_id::text, operator_name, host(client_addr), action, change \
         from events.audit_log where target_type = 'session' and at = $1 order by target_id",
    )
    .bind(at)
    .fetch_all(&pool)
    .await
    .unwrap();
    let entry = |n, operator: &str, name: &str, client: &str| {
        (
            session_id(n).to_string(),
            operator.to_owned(),
            name.to_owned(),
            client.to_owned(),
            "end".to_owned(),
            json!({ "reason": "hub_shutdown" }),
        )
    };
    assert_eq!(
        audits,
        [
            entry(1, OPERATOR_ID, "operator", "10.10.0.77"),
            entry(3, ALICE, "alice", "192.0.2.9"),
        ]
    );
    assert_eq!(
        event_count(&pool).await,
        0,
        "audit only: no tunnel ended (D-13)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_10_shut_down_ends_every_session_the_hub_holds_hub_shutdown_once(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01, DOCKER_HOST, OLLAMA]).await;
    let pending = started(&schema, PVE_01).await;
    let spent = started(&schema, DOCKER_HOST).await;
    let elsewhere = ClientAddr("10.10.0.99".parse().unwrap());
    assert!(
        live.redeem(
            Presented::terminal(spent["token"].as_str().unwrap(), elsewhere),
            now()
        )
        .is_err()
    );
    let opened = started(&schema, OLLAMA).await;
    let tunnel = redeem(&live, &opened, now()).unwrap().tunnel;
    let tunnel_task = tokio::spawn(stand_in_tunnel(pool.clone(), tunnel, id_of(&opened)));
    let at = now() + TimeDelta::seconds(5);

    sessions::shut_down(&pool, &live, at).await.unwrap();

    // The stand-in ends its row before it deregisters, so the row shows shut_down waited.
    let opened_row = stored(&pool, &id_of(&opened)).await;
    assert_eq!(opened_row.end_reason.as_deref(), Some("hub_shutdown"));
    assert_eq!(tunnel_task.await.unwrap(), EndReason::HubShutdown);
    for ticket in [&pending, &spent] {
        let row = stored(&pool, &id_of(ticket)).await;
        assert_eq!(row.ended_at, Some(at));
        assert_eq!(row.end_reason.as_deref(), Some("hub_shutdown"));
        let audits = end_audits(&pool, &id_of(ticket)).await;
        assert_eq!(audits.len(), 1, "{ticket}");
        assert_eq!(audits[0].change, json!({ "reason": "hub_shutdown" }));
        assert_eq!(audits[0].operator_name, "operator");
        assert_eq!(audits[0].client, CLIENT_ADDR);
    }
    assert!(
        end_audits(&pool, &id_of(&opened)).await.is_empty(),
        "the tunnel writes its own end entry"
    );
    assert_eq!(event_count(&pool).await, 0, "ticket ends are audit only");
    assert!(
        redeem(&live, &pending, at).is_none(),
        "the ticket is withdrawn"
    );

    // Shutting down again finds nothing left to end.
    sessions::shut_down(&pool, &live, at + TimeDelta::seconds(1))
        .await
        .unwrap();
    assert_eq!(
        session_audits(&pool).await.len(),
        5,
        "three starts, two ends"
    );
}

fn id_of(ticket: &Value) -> String {
    ticket["sessionId"].as_str().unwrap().to_owned()
}

/// A stand-in for the tunnel task: on the signal it ends its row, then deregisters.
async fn stand_in_tunnel(pool: PgPool, mut tunnel: Tunnel, id: String) -> EndReason {
    let cause = tunnel.ended().await;
    sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'hub_shutdown' \
         where id = $1::text::uuid and ended_at is null",
    )
    .bind(&id)
    .bind(now() + TimeDelta::seconds(5))
    .execute(&pool)
    .await
    .unwrap();
    drop(tunnel);
    cause.reason
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn mv_003_close_stale_ends_a_row_started_ahead_of_the_clock_at_its_start(pool: PgPool) {
    // As after the clock steps back: one row started 5 minutes "later" than now.
    insert_all(
        &pool,
        &[Row::live(1, PVE_01, 5), Row::live(2, DOCKER_HOST, -10)],
    )
    .await;

    assert_eq!(sessions::close_stale(&pool, now()).await.unwrap(), 2);

    let ahead = stored(&pool, &session_id(1).to_string()).await;
    assert_eq!(ahead.ended_at, Some(now() + TimeDelta::minutes(5)));
    assert_eq!(ahead.end_reason.as_deref(), Some("hub_shutdown"));
    let behind = stored(&pool, &session_id(2).to_string()).await;
    assert_eq!(behind.ended_at, Some(now()));
    for n in [1, 2] {
        let audits = end_audits(&pool, &session_id(n).to_string()).await;
        assert_eq!(audits.len(), 1, "session {n}");
    }
}

/// The advisory lock the test holds to keep a start's commit waiting.
const COMMIT_HOLD: i64 = 4210;

/// Makes every commit that inserted a session row wait for `COMMIT_HOLD`.
async fn hold_commits_on_lock(pool: &PgPool) {
    sqlx::raw_sql(
        "create function public.hold_commit() returns trigger language plpgsql as $$ \
         begin perform pg_advisory_xact_lock(4210); return null; end $$; \
         create constraint trigger hold_commit after insert on sessions.sessions \
             deferrable initially deferred for each row execute function public.hold_commit();",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Waits until some transaction waits for `COMMIT_HOLD`.
async fn commit_waiting(pool: &PgPool) {
    let waiting = async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "select exists (select from pg_locks where locktype = 'advisory' \
                 and objid = $1 and not granted)",
            )
            .bind(COMMIT_HOLD)
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), waiting)
        .await
        .expect("the start reaches its commit");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn d_10_a_start_that_commits_after_shut_down_ends_its_row_and_is_unavailable(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    hold_commits_on_lock(&pool).await;
    let mut holder = pool.acquire().await.unwrap();
    sqlx::query("select pg_advisory_lock($1)")
        .bind(COMMIT_HOLD)
        .execute(&mut *holder)
        .await
        .unwrap();
    let live = Live::default();
    let mut changes = live.changes();
    let schema = schema_at(&pool, now(), &live);
    let starting =
        tokio::spawn(async move { schema.execute(start_request(PVE_01, &display())).await });
    commit_waiting(&pool).await;

    // The ticket is issued and the row not yet visible: shut_down takes the ticket only.
    sessions::shut_down(&pool, &live, now()).await.unwrap();
    sqlx::query("select pg_advisory_unlock($1)")
        .bind(COMMIT_HOLD)
        .execute(&mut *holder)
        .await
        .unwrap();
    let response = starting.await.unwrap();

    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let error = serde_json::to_value(&response.errors[0]).unwrap();
    assert_eq!(error["extensions"]["code"], "UNAVAILABLE", "{error}");
    let id: uuid::Uuid = sqlx::query_scalar("select id from sessions.sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    let row = stored(&pool, &id.to_string()).await;
    assert_eq!(row.end_reason.as_deref(), Some("hub_shutdown"));
    let audits = end_audits(&pool, &id.to_string()).await;
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].change, json!({ "reason": "hub_shutdown" }));
    // The refused start is never published; the end of its row is.
    let change = changes.try_recv().unwrap().session;
    assert_eq!((change.id.to_string(), change.is_live()), (id.to_string(), false));
    assert!(changes.try_recv().is_err(), "one change only");
}
