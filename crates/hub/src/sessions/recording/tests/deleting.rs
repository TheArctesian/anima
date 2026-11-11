//! `deleteRecording` (K1, D-13, D-15, G-8, LOG-005, API-003): the row and its key go first with
//! the audit entry `recording.delete`, then the file; a file already gone is fine; a delete
//! that does not commit leaves the file; an unknown id changes nothing; and every catalogue
//! field needs a signed-in operator.

use std::path::Path;

use async_graphql::Request;
use chrono::TimeDelta;
use serde_json::{Value as Json, json};
use sqlx::PgPool;

use super::{
    World,
    catalogue::{CLIENT, FILE_BYTES, asked, catalogue, data, error_of, shelve},
    viewing::{audits, refuse_audits_at_commit},
};
use crate::{
    access::Identity,
    ids::RecordingId,
    sessions::{
        Recording,
        recording::store,
        tests::fixture::{docker_host, now, operator},
    },
};

const DELETE: &str = "mutation($id: ID!) { deleteRecording(id: $id) }";

async fn delete(world: &World, id: &str) -> async_graphql::Response {
    asked(world, DELETE, json!({ "id": id })).await
}

async fn row_exists(world: &World, id: RecordingId) -> bool {
    store::catalogued(&world.serving.db, id)
        .await
        .unwrap()
        .is_some()
}

/// `(severity, source, device name, message)` of each event.
async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as("select severity, source, device_name, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_13_delete_removes_the_row_then_the_file_and_writes_recording_delete(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let kept = shelve(&world, docker_host(), 1).await;
    let session = world.stored(id).await.session;

    let found = data(delete(&world, &id.to_string()).await);
    assert_eq!(found, json!({ "deleteRecording": id.to_string() }));
    assert!(!row_exists(&world, id).await, "the key is gone");
    assert!(!world.path(id).exists(), "the file is gone");
    assert!(row_exists(&world, kept).await && world.path(kept).exists());

    let change = json!({
        "sessionId": session.to_string(),
        "deviceName": "docker-host",
        "protocol": "vnc",
        "startedAt": now(),
    });
    let entry = (
        "operator".to_owned(),
        "delete".to_owned(),
        "recording".to_owned(),
        id.to_string(),
        CLIENT.to_owned(),
        change,
    );
    assert_eq!(audits(&pool).await, [entry]);
    let event = (
        "info".to_owned(),
        "session".to_owned(),
        Some("docker-host".to_owned()),
        "A session recording was deleted. Its key is gone, so its file cannot be read again."
            .to_owned(),
    );
    assert_eq!(events(&pool).await, [event]);
    let sessions: i64 = sqlx::query_scalar("select count(*) from sessions.sessions where id = $1")
        .bind(session)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 1, "the session's own row stays (RA-008)");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_8_a_file_already_gone_is_fine(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    std::fs::remove_file(world.path(id)).unwrap();
    let found = data(delete(&world, &id.to_string()).await);
    assert_eq!(found["deleteRecording"], id.to_string());
    assert!(!row_exists(&world, id).await);
    assert_eq!(audits(&pool).await.len(), 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_8_a_file_that_cannot_be_removed_still_leaves_no_key(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    // A folder in the file's place: `remove_file` fails, and the row is already gone.
    std::fs::remove_file(world.path(id)).unwrap();
    std::fs::create_dir(world.path(id)).unwrap();
    let found = data(delete(&world, &id.to_string()).await);
    assert_eq!(found["deleteRecording"], id.to_string());
    assert!(!row_exists(&world, id).await);
    assert!(world.path(id).is_dir(), "left for the retention sweep");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_8_a_delete_that_does_not_commit_leaves_the_row_and_the_file(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    refuse_audits_at_commit(&pool).await;
    let (code, message) = error_of(&delete(&world, &id.to_string()).await);
    assert_eq!(code, "INTERNAL");
    assert!(!message.contains(&id.to_string()) && !message.contains(".anrec"));
    assert!(row_exists(&world, id).await, "rolled back");
    assert_eq!(
        std::fs::read(world.path(id)).unwrap(),
        FILE_BYTES,
        "the file stays"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k1_deleting_an_unknown_or_malformed_recording_changes_nothing(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let kept = shelve(&world, docker_host(), 0).await;
    let unknown = RecordingId::from(uuid::Uuid::now_v7());
    // A file with the unknown id's name: only a deleted row lets its file go.
    std::fs::write(world.path(unknown), FILE_BYTES).unwrap();
    let (code, message) = error_of(&delete(&world, &unknown.to_string()).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such recording {unknown}."));
    assert!(world.path(unknown).exists());
    for malformed in ["", "x", "../kept", &format!("{kept}/..")] {
        let (code, _) = error_of(&delete(&world, malformed).await);
        assert_eq!(code, "VALIDATION", "{malformed}");
    }
    assert!(row_exists(&world, kept).await && world.path(kept).exists());
    assert_eq!(audits(&pool).await, []);
    assert_eq!(events(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_13_with_no_recording_root_the_row_still_goes(pool: PgPool) {
    let mut world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    world.serving.recording = Recording {
        root: None,
        max_bytes: 1,
    };
    data(delete(&world, &id.to_string()).await);
    assert!(!row_exists(&world, id).await);
    assert!(
        Path::new(&world.path(id)).exists(),
        "no root, no file touched"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_003_every_catalogue_field_needs_a_signed_in_operator(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let documents = [
        ("{ recordings { edges { node { id } } } }", json!({})),
        (
            "mutation($id: ID!) { openRecording(id: $id) { token } }",
            json!({ "id": id.to_string() }),
        ),
        (DELETE, json!({ "id": id.to_string() })),
    ];
    let schema = catalogue(&world);
    for (document, variables) in documents {
        let signed_out = Request::new(document)
            .variables(async_graphql::Variables::from_json(variables.clone()));
        let expired = Identity::for_tests(operator(), now() - TimeDelta::seconds(1)).attach(
            Request::new(document).variables(async_graphql::Variables::from_json(variables)),
        );
        for request in [signed_out, expired] {
            let response = schema.execute(request).await;
            assert_eq!(error_of(&response).0, "UNAUTHENTICATED", "{document}");
        }
    }
    assert!(row_exists(&world, id).await && world.path(id).exists());
    assert_eq!(
        audits(&pool).await,
        Vec::<(String, String, String, String, String, Json)>::new()
    );
    assert_eq!(world.serving.live.playback().len(), 0);
}
