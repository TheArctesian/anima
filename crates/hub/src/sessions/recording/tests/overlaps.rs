//! An open and a delete of one recording that overlap (G-7, K4 §12, D-13): the open holds the
//! row from its read until its ticket and `recording.view` commit, so the delete waits for it
//! and then withdraws that ticket. Locks, not timing, order the two. The open's lock blocks
//! only a delete: an update of the row (the writer's end, the expiry job) goes on.

use std::cell::Cell;

use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{
    World,
    catalogue::{CLIENT, asked, data, shelve},
    deleting::{delete, row_exists},
    eventually,
    viewing::audits,
};
use crate::{
    events::{self, AuditAction, AuditTarget, NewAudit},
    sessions::{
        Actor,
        recording::viewing::open_recording,
        tests::fixture::{address, docker_host, now, operator},
    },
};

/// The advisory lock the first audit write waits on, held by the test.
const GATE: i64 = 4_802;

/// Makes the first audit write wait, inside its transaction, until the test lets [`GATE`] go.
async fn gate_the_first_audit_write(pool: &PgPool) {
    let gated = format!(
        "create function events.ensure_day_partition(parent text, at timestamptz) \
         returns void language plpgsql as $$ begin \
           if nextval('public.first_write') = 1 then \
             perform pg_advisory_lock({GATE}); perform pg_advisory_unlock({GATE}); \
           end if; \
           perform events.ensure_day_partition_real(parent, at); end $$"
    );
    for statement in [
        "alter function events.ensure_day_partition(text, timestamptz) \
         rename to ensure_day_partition_real",
        "create sequence public.first_write",
        &gated,
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }
}

/// Whether a backend of this test's database waits on a lock of a kind in `wait_events`,
/// running a statement that starts with `statement`.
async fn waits(pool: &PgPool, wait_events: &[&str], statement: &str) -> bool {
    sqlx::query_scalar(
        "select exists (select 1 from pg_stat_activity where datname = current_database() \
           and wait_event_type = 'Lock' and wait_event = any($1) and starts_with(query, $2))",
    )
    .bind(wait_events)
    .bind(statement)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_7_a_delete_that_overlaps_an_open_waits_for_it_and_withdraws_its_ticket(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    gate_the_first_audit_write(&pool).await;
    let mut gate = pool.acquire().await.unwrap();
    sqlx::query("select pg_advisory_lock($1)")
        .bind(GATE)
        .execute(&mut *gate)
        .await
        .unwrap();
    let open = "mutation($id: ID!) { openRecording(id: $id) { token } }";
    let delete_done = Cell::new(false);
    // The open has read the row and waits at its audit write; only then does the delete start.
    let opening = asked(&world, open, json!({ "id": id.to_string() }));
    let deleting = async {
        eventually("the open waits at its audit write", async || {
            waits(&pool, &["advisory"], "select events.ensure_day_partition").await
        })
        .await;
        let deleted = delete(&world, &id.to_string()).await;
        delete_done.set(true);
        deleted
    };
    // The delete either waits for the open's row, or (without the lock) has already finished.
    let releasing = async {
        eventually("the delete waits for the open or is done", async || {
            delete_done.get() || waits(&pool, &["transactionid", "tuple"], "delete from").await
        })
        .await;
        sqlx::query("select pg_advisory_unlock($1)")
            .bind(GATE)
            .execute(&mut *gate)
            .await
            .unwrap();
    };
    let (opened, deleted, ()) = tokio::join!(opening, deleting, releasing);
    let token = data(opened)["openRecording"]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(data(deleted)["deleteRecording"], id.to_string());
    assert!(!row_exists(&world, id).await);
    let playback = world.serving.live.playback();
    assert_eq!(playback.len(), 0, "no ticket outlives the delete");
    assert!(playback.redeem(&token, address(CLIENT), now()).is_err());
    let actions: Vec<String> = audits(&pool).await.into_iter().map(|a| a.1).collect();
    assert_eq!(
        actions,
        ["view", "delete"],
        "the view is written before the delete"
    );
}

/// A transaction that wrote an audit entry and then updates the row is not blocked by an open
/// that waits to create its audit day partition: the open's `for key share` does not conflict
/// with an update, so no deadlock forms (G-7).
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_7_an_open_does_not_block_an_update_of_its_row(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let mut writer = pool.begin().await.unwrap();
    let entry = NewAudit {
        operator: operator(),
        action: AuditAction::View,
        target: AuditTarget::Recording(id),
        client: address(CLIENT),
        change: json!({}),
    };
    events::audit(&mut writer, now(), entry).await.unwrap();
    let by = Actor {
        operator: operator(),
        client: address(CLIENT),
        at: now() + TimeDelta::days(20),
    };
    let opening = open_recording(&pool, &world.serving.live, &by, id);
    let updating = async {
        eventually("the open waits to create its partition", async || {
            sqlx::query_scalar::<_, bool>(
                "select exists (select 1 from pg_stat_activity \
                 where datname = current_database() and wait_event_type = 'Lock' \
                 and starts_with(query, 'select events.ensure_day_partition'))",
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        })
        .await;
        sqlx::query("update sessions.recordings set expires_at = expires_at where id = $1")
            .bind(id)
            .execute(&mut *writer)
            .await?;
        writer.commit().await
    };
    let (opened, updated) = tokio::join!(opening, updating);
    assert!(updated.is_ok(), "the update failed: {updated:?}");
    assert!(opened.is_ok(), "the open failed: {:?}", opened.err());
}
