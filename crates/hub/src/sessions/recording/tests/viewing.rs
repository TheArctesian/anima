//! `openRecording` (K1, K4 §1 and §12, RA-041, LOG-005, LOG-007): a single-use ticket for
//! `/recording`, named for the recording's session, with the audit entry `recording.view`;
//! nothing for an unknown id, and no ticket without its entry.

use chrono::{DateTime, Utc};
use serde_json::{Value as Json, json};
use sqlx::PgPool;

use super::{
    World,
    catalogue::{CLIENT, asked, data, error_of, shelve},
};
use crate::{
    ids::RecordingId,
    sessions::{
        Presented,
        recording::tickets::MAX_PENDING,
        tests::fixture::{address, docker_host, now},
        tickets::LIFETIME,
    },
};

const OPEN: &str = "mutation($id: ID!) {
    openRecording(id: $id) { sessionId token tunnelPath expiresAt wakeSent }
}";

async fn open(world: &World, id: &str) -> async_graphql::Response {
    asked(world, OPEN, json!({ "id": id })).await
}

/// `(operator, action, target type, target id, client, change)` of each audit entry.
pub(super) async fn audits(pool: &PgPool) -> Vec<(String, String, String, String, String, Json)> {
    sqlx::query_as(
        "select operator_name, action, target_type, target_id, host(client_addr), change \
         from events.audit_log order by at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_12_open_recording_issues_a_recording_ticket_and_writes_recording_view(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let session = world.stored(id).await.session;

    let found = data(open(&world, &id.to_string()).await);
    let ticket = &found["openRecording"];
    assert_eq!(ticket["sessionId"], session.to_string());
    assert_eq!(ticket["tunnelPath"], "/recording");
    assert_eq!(ticket["wakeSent"], false);
    let expires_at: DateTime<Utc> = ticket["expiresAt"].as_str().unwrap().parse().unwrap();
    assert_eq!(expires_at, now() + LIFETIME);
    let token = ticket["token"].as_str().unwrap();
    assert_eq!(token.len(), 43);

    let entry = (
        "operator".to_owned(),
        "view".to_owned(),
        "recording".to_owned(),
        id.to_string(),
        CLIENT.to_owned(),
        json!({ "sessionId": session.to_string(), "deviceName": "docker-host", "protocol": "vnc" }),
    );
    assert_eq!(audits(&pool).await, [entry]);

    let live = &world.serving.live;
    let session_tunnel = Presented {
        token,
        client: address(CLIENT),
        path: "/tunnel",
        tunnel_path: crate::sessions::Protocol::tunnel_path,
    };
    assert!(
        live.redeem(session_tunnel, now()).is_err(),
        "a playback ticket opens no session tunnel"
    );
    let redeemed = live
        .playback()
        .redeem(token, address(CLIENT), now())
        .unwrap();
    assert_eq!(
        (redeemed.recording, redeemed.session, redeemed.client),
        (id, session, address(CLIENT))
    );
    assert_eq!(redeemed.operator.name, "operator");
    assert_eq!(redeemed.expires_at, now() + LIFETIME);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_12_each_open_is_its_own_ticket_and_its_own_entry(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let first = data(open(&world, &id.to_string()).await);
    let second = data(open(&world, &id.to_string()).await);
    assert_ne!(
        first["openRecording"]["token"],
        second["openRecording"]["token"]
    );
    assert_eq!(world.serving.live.playback().len(), 2);
    assert_eq!(audits(&pool).await.len(), 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k1_an_unknown_or_malformed_recording_is_refused_with_nothing_written(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    shelve(&world, docker_host(), 0).await;
    let unknown = RecordingId::from(uuid::Uuid::now_v7());
    let (code, message) = error_of(&open(&world, &unknown.to_string()).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such recording {unknown}."));
    for malformed in [
        "",
        "not-a-uuid",
        "../../etc/passwd",
        "0190a000-0000-7000-8002",
    ] {
        let (code, _) = error_of(&open(&world, malformed).await);
        assert_eq!(code, "VALIDATION", "{malformed}");
    }
    assert_eq!(audits(&pool).await, []);
    assert_eq!(world.serving.live.playback().len(), 0, "no ticket held");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_with_every_playback_ticket_pending_the_open_is_unavailable_and_unaudited(
    pool: PgPool,
) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    for _ in 0..MAX_PENDING {
        data(open(&world, &id.to_string()).await);
    }
    let (code, message) = error_of(&open(&world, &id.to_string()).await);
    assert_eq!(code, "UNAVAILABLE");
    assert_eq!(
        message,
        "The hub cannot open a recording now. Try again in a moment."
    );
    assert_eq!(
        audits(&pool).await.len(),
        MAX_PENDING,
        "the refused open has no entry"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_10_a_hub_shutting_down_issues_no_playback_ticket(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    let _closing = world.serving.live.shut_down();
    let (code, _) = error_of(&open(&world, &id.to_string()).await);
    assert_eq!(code, "UNAVAILABLE");
    assert_eq!(audits(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_005_an_entry_that_cannot_be_written_leaves_no_ticket(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let id = shelve(&world, docker_host(), 0).await;
    // The commit fails after the ticket is issued: a deferred check on the audit log.
    refuse_audits_at_commit(&pool).await;
    let (code, _) = error_of(&open(&world, &id.to_string()).await);
    assert_eq!(code, "INTERNAL");
    assert_eq!(world.serving.live.playback().len(), 0, "withdrawn");
}

/// Makes every commit that wrote an audit entry fail, with a deferred constraint trigger.
pub(super) async fn refuse_audits_at_commit(pool: &PgPool) {
    sqlx::query(
        "create or replace function public.refuse_at_commit() returns trigger \
         language plpgsql as $$ begin raise exception 'refused at commit'; end $$",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "create constraint trigger refuse_at_commit after insert on events.audit_log \
         deferrable initially deferred for each row execute function public.refuse_at_commit()",
    )
    .execute(pool)
    .await
    .unwrap();
}
