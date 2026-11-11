//! Expiry in the catalogue (D-25, CR-23 e, RA-041): an ended recording past its `expires_at`
//! is neither listed nor opened, though its row waits for the daily retention job and may
//! still be deleted; a live recording stays listed and openable past it, as the job keeps it
//! until it ends. A recording ends with its session even when its own end was never written.

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{
    World,
    catalogue::{asked, data, error_of, shelve},
    viewing::audits,
};
use crate::{
    ids::RecordingId,
    sessions::{
        recording::store,
        tests::fixture::{docker_host, now},
    },
};

const LIST: &str = "{ recordings { edges { node { id } } } }";
const DEVICE_LIST: &str = "query($device: ID) {
    recordings(deviceId: $device) { edges { node { id } } }
}";
const OPEN: &str = "mutation($id: ID!) { openRecording(id: $id) { token } }";
const DELETE: &str = "mutation($id: ID!) { deleteRecording(id: $id) }";

/// Sets when the recording `id` ended (`None`: it is live) and when it expires, as the
/// recorder and the retention job would.
async fn age(
    world: &World,
    id: RecordingId,
    ended_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
) {
    sqlx::query("update sessions.recordings set ended_at = $2, expires_at = $3 where id = $1")
        .bind(id)
        .bind(ended_at)
        .bind(expires_at)
        .execute(&world.serving.db)
        .await
        .unwrap();
}

/// The ids `document` lists, newest first.
async fn listed(world: &World, document: &str, variables: serde_json::Value) -> Vec<String> {
    let found = data(asked(world, document, variables).await);
    found["recordings"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Three recordings at the clock's `now`: ended and expired exactly now, ended and expiring a
/// microsecond later, and live but expired a day ago.
async fn three(world: &World) -> (RecordingId, RecordingId, RecordingId) {
    let expired = shelve(world, docker_host(), 0).await;
    let kept = shelve(world, docker_host(), 1).await;
    let live = shelve(world, docker_host(), 2).await;
    age(world, expired, Some(now()), now()).await;
    let micro = TimeDelta::microseconds(1);
    age(world, kept, Some(now()), now() + micro).await;
    age(world, live, None, now() - TimeDelta::days(1)).await;
    (expired, kept, live)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_25_an_ended_expired_recording_is_not_listed_and_a_live_one_is(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let (expired, kept, live) = three(&world).await;
    let shown = [live.to_string(), kept.to_string()];
    assert_eq!(listed(&world, LIST, json!({})).await, shown);
    let device = json!({ "device": docker_host().to_string() });
    assert_eq!(listed(&world, DEVICE_LIST, device).await, shown);
    assert!(
        store::get(&pool, expired).await.unwrap().is_some(),
        "the row waits for the retention job"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_25_an_ended_expired_recording_cannot_be_opened_and_a_live_one_can(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let (expired, kept, live) = three(&world).await;
    let (code, message) =
        error_of(&asked(&world, OPEN, json!({ "id": expired.to_string() })).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such recording {expired}."));
    assert_eq!(audits(&pool).await, [], "a refused open has no entry");
    assert_eq!(world.serving.live.playback().len(), 0, "and no ticket");

    for id in [kept, live] {
        data(asked(&world, OPEN, json!({ "id": id.to_string() })).await);
    }
    assert_eq!(world.serving.live.playback().len(), 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_13_an_ended_expired_recording_can_still_be_deleted(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let (expired, _, _) = three(&world).await;
    let found = data(asked(&world, DELETE, json!({ "id": expired.to_string() })).await);
    assert_eq!(found["deleteRecording"], expired.to_string());
    assert!(store::get(&pool, expired).await.unwrap().is_none());
    assert!(!world.path(expired).exists());
}

/// Ends the session of the recording `id` as the hub does, and leaves the recording's own end
/// unwritten, as a failed end or a hub crash before `close_stale` would.
async fn end_session(world: &World, id: RecordingId) {
    sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'hub_shutdown' \
         where id = (select session_id from sessions.recordings where id = $1)",
    )
    .bind(id)
    .bind(now())
    .execute(&world.serving.db)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn cr_23_e_an_unended_expired_recording_of_an_ended_session_is_neither_listed_nor_opened(
    pool: PgPool,
) {
    let world = World::new(&pool, 1).await;
    let (_, kept, live) = three(&world).await;
    end_session(&world, live).await;
    assert_eq!(listed(&world, LIST, json!({})).await, [kept.to_string()]);
    let (code, _) = error_of(&asked(&world, OPEN, json!({ "id": live.to_string() })).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(audits(&pool).await, [], "a refused open has no entry");
    assert_eq!(world.serving.live.playback().len(), 0, "and no ticket");
    assert!(store::playable(&pool, live, now()).await.unwrap().is_none());
    let stored = store::get(&pool, live).await.unwrap().unwrap();
    assert_eq!(stored.ended_at, None, "the row waits for the retention job");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn cr_23_e_an_unended_expired_recording_of_a_live_session_stays_listed_and_playable(
    pool: PgPool,
) {
    let world = World::new(&pool, 1).await;
    let (expired, kept, live) = three(&world).await;
    // Another session's end ends no other recording.
    sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'operator' where id = $1",
    )
    .bind(world.origin.session)
    .bind(now())
    .execute(&pool)
    .await
    .unwrap();
    let shown = [live.to_string(), kept.to_string()];
    assert_eq!(listed(&world, LIST, json!({})).await, shown);
    assert!(store::playable(&pool, live, now()).await.unwrap().is_some());
    assert!(
        store::playable(&pool, expired, now())
            .await
            .unwrap()
            .is_none()
    );
    data(asked(&world, OPEN, json!({ "id": live.to_string() })).await);
    assert_eq!(world.serving.live.playback().len(), 1);
}
