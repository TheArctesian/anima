//! Stops (K4 §11, G-5): at the size limit, under 1 GiB free, on a write error or a full queue.
//! Each writes the last part when the file still can take it, ends the row with its reason,
//! publishes `sessionChanged` and writes a Warning event with a fixed sentence; the session
//! goes on and later bytes are dropped. Nothing names the file's path (LOG-007).

use std::{fmt, os::unix::fs::PermissionsExt as _};

use sqlx::PgPool;
use static_assertions::assert_not_impl_any;

use super::{Ended, FAILING, LOW, PLENTY, World};
use crate::{
    events::process_log,
    sessions::{
        SessionChange,
        recording::{
            Recorder, RecordingState, Stored,
            format::{MAX_PART, RecordingKey, Sealer},
            model::StopReason,
            states,
        },
        tests::fixture::{Capture, stored_text},
    },
};

assert_not_impl_any!(RecordingKey: fmt::Debug, fmt::Display, Clone);
assert_not_impl_any!(Sealer: fmt::Debug, fmt::Display, Clone);
assert_not_impl_any!(Recorder: fmt::Debug, fmt::Display, Clone);
assert_not_impl_any!(Stored: fmt::Debug, fmt::Display, Clone);

/// The one Warning event's message, with its severity and source checked.
async fn warning(pool: &PgPool) -> String {
    let rows: Vec<(String, String, Option<String>, String)> =
        sqlx::query_as("select severity, source, device_name, message from events.event_log")
            .fetch_all(pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 1, "one event: {rows:?}");
    let (severity, source, device, message) = rows.into_iter().next().unwrap();
    assert_eq!((severity.as_str(), source.as_str()), ("warning", "session"));
    assert_eq!(device.as_deref(), Some("docker-host"));
    message
}

/// Checks the stop of `world`'s recording `id`: the row, the state, the change and the event.
async fn stopped(world: &World, changes: &mut tokio::sync::broadcast::Receiver<SessionChange>) {
    let change = changes.try_recv().expect("a stop publishes sessionChanged");
    assert_eq!(change.session.id, world.origin.session);
    assert!(change.session.is_live(), "the session goes on");
    let found = states(&world.serving.db, &[world.origin.session])
        .await
        .unwrap();
    assert_eq!(found[&world.origin.session], RecordingState::Stopped);
    let message = warning(&world.serving.db).await;
    let root = world.root.path().to_str().unwrap();
    assert!(!message.contains(root), "{message}");
    assert!(!stored_text(&world.serving.db).await.contains(root));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_size_limit_keeps_exactly_max_bytes_then_drops(pool: PgPool) {
    let world = World::new(&pool, 10).await;
    let mut changes = world.serving.live.changes();
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    recorder.push(b"0123");
    recorder.push(b"456789");
    recorder.push(b"x");
    recorder.push(b"later");
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!(
        (plaintext.as_slice(), ended),
        (&b"0123456789"[..], Ended::Done)
    );
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::SizeLimit));
    assert_eq!(stored.size_bytes, 10);
    assert!(stored.complete, "a stop writes the last part");
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_under_1_gib_free_stops_disk_full_with_the_last_part(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    let recorder = world.start(LOW).await;
    let id = recorder.id();
    recorder.push(&vec![1; MAX_PART]);
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (0, Ended::Done));
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::DiskFull));
    assert!(stored.complete);
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_under_1_gib_free_before_the_last_part_drops_its_bytes(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    let recorder = world.start(LOW).await;
    let id = recorder.id();
    recorder.push(b"3.nop;");
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (0, Ended::Done));
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::DiskFull));
    assert_eq!((stored.complete, stored.size_bytes), (true, 0));
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_failed_reading_before_the_last_part_stops_write_failed_without_it(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    let recorder = world.start(FAILING).await;
    let id = recorder.id();
    recorder.push(b"3.nop;");
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (0, Ended::Incomplete));
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::WriteFailed));
    assert_eq!((stored.complete, stored.size_bytes), (false, 0));
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_5_a_stop_ends_the_row_only_with_its_warning_event(pool: PgPool) {
    // The event log refuses every row, so the stop's transaction cannot commit.
    for statement in [
        "create function public.refuse() returns trigger language plpgsql as \
         $$ begin raise exception 'refused'; end $$",
        "create trigger refuse before insert on events.event_log \
         for each row execute function public.refuse()",
    ] {
        sqlx::query(statement).execute(&pool).await.unwrap();
    }
    let world = World::new(&pool, 10).await;
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    recorder.push(b"0123456789ab");
    recorder.finish().await;

    let stored = world.stored(id).await;
    assert_eq!(stored.ended_at, None, "the end rolled back with the event");
    assert_eq!(stored.stopped, None);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_write_error_stops_write_failed_without_a_last_part(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    let recorder = world.start(FAILING).await;
    let id = recorder.id();
    recorder.push(&vec![1; MAX_PART]);
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (0, Ended::Incomplete));
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::WriteFailed));
    assert!(
        !stored.complete,
        "a file that failed a write gets no last part"
    );
    assert_eq!(stored.ended_at, Some(stored.started_at));
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_full_queue_stops_write_failed_without_the_relay_waiting(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    // The disk then refuses the kept parts, so this debug build seals no 4 MiB; the queue's
    // tests show kept parts are written before the last.
    let recorder = world.start(LOW).await;
    let id = recorder.id();
    // `push` is not async and this test does not yield between pushes, so the writer, on
    // this same thread, takes no part until the 65th push has stopped the recording. 64 is
    // K4 §11's bound, written out so a changed constant fails here.
    for _ in 0..=64 {
        recorder.push(&vec![2; MAX_PART]);
    }
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (0, Ended::Done));
    let stored = world.stored(id).await;
    assert_eq!(
        stored.stopped,
        Some(StopReason::WriteFailed),
        "the first stop's reason stays"
    );
    assert!(stored.complete);
    stopped(&world, &mut changes).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_4_a_file_that_cannot_be_created_leaves_the_row_stopped(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let mut changes = world.serving.live.changes();
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in the Guacamole tunnel's test: a second registered dispatcher makes every callsite
    // registration ask all of them, so another test thread cannot disable these callsites.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let root = world.root.path();
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o500)).unwrap();
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    recorder.push(b"3.nop;");
    recorder.finish().await;
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert!(!world.path(id).exists());
    let stored = world.stored(id).await;
    assert_eq!(stored.stopped, Some(StopReason::WriteFailed));
    assert_eq!((stored.complete, stored.size_bytes), (false, 0));
    stopped(&world, &mut changes).await;
    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("a recording stopped"), "{log}");
    assert!(!log.contains(root.to_str().unwrap()), "{log}");
    assert!(!log.contains(".anrec"), "{log}");
}
