//! The row (K2, G-5, G-6): a session's recording state, an end written once, the expiry from
//! the retention period, and what the runtime role may change.

use chrono::{Days, TimeDelta};
use sqlx::PgPool;
use uuid::Uuid;

use super::{PLENTY, World, new_session};
use crate::{
    credentials::Sealed,
    ids::{RecordingId, SessionId},
    sessions::{
        Protocol,
        recording::{
            RecordingState,
            format::Format,
            model::StopReason,
            states,
            store::{self, Ending, NewRecording},
        },
        tests::fixture::{docker_host, now},
    },
};

fn sealed() -> Sealed {
    Sealed::from_parts(&[1; 24], vec![2; 48], "k1".to_owned()).unwrap()
}

async fn insert(conn: &mut sqlx::PgConnection, session: SessionId) -> RecordingId {
    let id = RecordingId::from(Uuid::now_v7());
    let key_sealed = sealed();
    let new = NewRecording {
        id,
        session,
        device: docker_host(),
        device_name: "docker-host",
        protocol: Protocol::Vnc,
        format: Format::Guacamole,
        key_sealed: &key_sealed,
        started_at: now(),
        expires_at: now() + Days::new(30),
    };
    store::insert(&mut *conn, &new).await.unwrap();
    id
}

fn ending(minutes: i64, stopped: Option<StopReason>) -> Ending {
    Ending {
        at: now() + TimeDelta::minutes(minutes),
        size_bytes: 3_000_000_000,
        complete: true,
        stopped,
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_5_a_session_shows_off_on_or_stopped(pool: PgPool) {
    let on = World::new(&pool, 1).await;
    let mut conn = pool.acquire().await.unwrap();
    let id = insert(&mut conn, on.origin.session).await;
    let stopped = new_session(&pool).await;
    let stopped_id = insert(&mut conn, stopped).await;
    store::end(
        &mut *conn,
        stopped_id,
        &ending(1, Some(StopReason::DiskFull)),
    )
    .await
    .unwrap();
    let ended_on = store::end(&mut *conn, id, &ending(2, None)).await.unwrap();
    assert!(ended_on, "a recorded session's end leaves it ON");
    let off = SessionId::from(Uuid::now_v7());

    let asked = [on.origin.session, stopped, off];
    let found = states(&pool, &asked).await.unwrap();
    assert_eq!(found.len(), 3, "every asked session has a state");
    assert_eq!(found[&on.origin.session], RecordingState::On);
    assert_eq!(found[&stopped], RecordingState::Stopped);
    assert_eq!(found[&off], RecordingState::Off);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_recording_ends_once_and_keeps_its_size(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let mut conn = pool.acquire().await.unwrap();
    let id = insert(&mut conn, world.origin.session).await;
    assert!(
        store::end(&mut *conn, id, &ending(5, Some(StopReason::SizeLimit)))
            .await
            .unwrap()
    );
    assert!(
        !store::end(&mut *conn, id, &ending(9, None)).await.unwrap(),
        "the first end wins"
    );

    let stored = world.stored(id).await;
    assert_eq!(stored.ended_at, Some(now() + TimeDelta::minutes(5)));
    assert_eq!(stored.size_bytes, 3_000_000_000, "sizes above 2^31 keep");
    assert_eq!(stored.stopped, Some(StopReason::SizeLimit));
    assert!(stored.complete);
    assert_eq!(stored.key_sealed, sealed());
    assert_eq!(stored.format, Format::Guacamole);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_6_expires_at_is_the_start_plus_the_recordings_period(pool: PgPool) {
    sqlx::query("update retention.periods set days = 7 where subject = 'recordings'")
        .execute(&pool)
        .await
        .unwrap();
    let world = World::new(&pool, 1).await;
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    recorder.finish().await;
    let stored = world.stored(id).await;
    assert_eq!(stored.started_at, now());
    assert_eq!(stored.expires_at, now() + Days::new(7));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn r_016_the_runtime_role_writes_recordings_but_never_replaces_a_key(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    // Detached: a connection whose role changed must not return to the pool (R-019).
    let mut conn = pool.acquire().await.unwrap().detach();
    sqlx::query("set role anima_runtime")
        .execute(&mut conn)
        .await
        .unwrap();
    let id = insert(&mut conn, world.origin.session).await;
    assert!(store::end(&mut conn, id, &ending(1, None)).await.unwrap());
    store::get(&mut conn, id).await.unwrap().unwrap();
    states(&mut conn, &[world.origin.session]).await.unwrap();

    let replaced = sqlx::query("update sessions.recordings set key_sealed = '\\x00' where id = $1")
        .bind(id)
        .execute(&mut conn)
        .await;
    let error = replaced.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("42501"),
        "permission denied"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k2_a_recorded_session_cannot_be_deleted(pool: PgPool) {
    let world = World::new(&pool, 1).await;
    let mut conn = pool.acquire().await.unwrap();
    insert(&mut conn, world.origin.session).await;
    let deleted = sqlx::query("delete from sessions.sessions where id = $1")
        .bind(world.origin.session)
        .execute(&pool)
        .await;
    let error = deleted.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503"),
        "on delete restrict"
    );
}
