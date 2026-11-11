//! The recorder writing real files (K4 §11, G-4): the row before the file, a new file with
//! mode 0600, parts that read back with the key from the row, a full part at once and the
//! open part 5 seconds after its first byte, and the end of the row.

use std::{os::unix::fs::PermissionsExt as _, task::Poll, time::Duration};

use sqlx::PgPool;
use uuid::Uuid;

use super::{EXACT, Ended, FRAMING, PLENTY, World, eventually, length};
use crate::{
    ids::{RecordingId, SessionId},
    sessions::{
        Protocol,
        recording::{
            Error, Recorder, RecordingState,
            format::{Format, HEADER_LEN, MAX_PART},
            recorder::{FLUSH_AFTER, begin, create_file, key_context},
            start, states,
        },
        tests::fixture::now,
    },
};

/// Gives the writer task and its blocking writes real time to run until `done` holds.
async fn until(done: impl Fn() -> bool) {
    for _ in 0..2000 {
        if done() {
            return;
        }
        tokio::task::yield_now().await;
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("the writer did not get there");
}

/// Lets the writer run a few turns, for an assertion that it wrote nothing.
async fn settle() {
    for _ in 0..20 {
        tokio::task::yield_now().await;
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_040_guacamole_instructions_read_back_with_the_key_from_the_row(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    // The hub's own free-space reading: the test folder has more than 1 GiB free.
    let recorder = start(&world.serving, world.origin.clone()).await.unwrap();
    let id = recorder.id();
    let instructions: [&[u8]; 3] = [b"4.size,1.0,4.1024,3.768;", b"4.sync,3.100;", b"3.nop;"];
    for instruction in instructions {
        recorder.push(instruction);
    }
    recorder.finish().await;

    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!(plaintext, instructions.concat());
    assert_eq!(ended, Ended::Done);
    let stored = world.stored(id).await;
    assert_eq!(stored.id, id);
    assert_eq!(stored.session, world.origin.session);
    assert_eq!(stored.device, Some(world.origin.device));
    assert_eq!(stored.protocol, Protocol::Vnc);
    assert_eq!(stored.format, Format::Guacamole);
    assert_eq!(stored.device_name, "docker-host");
    assert_eq!(stored.ended_at, Some(now()));
    assert_eq!(stored.size_bytes, instructions.concat().len() as u64);
    assert!(stored.complete);
    assert_eq!(stored.stopped, None);
    let found = states(&pool, &[world.origin.session]).await.unwrap();
    assert_eq!(found[&world.origin.session], RecordingState::On);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_the_file_is_new_and_only_the_hub_reads_it(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let recorder = world.start(PLENTY).await;
    let path = world.path(recorder.id());
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(
        length(&path),
        HEADER_LEN as u64,
        "the header, before any part"
    );
    recorder.finish().await;
}

#[test]
fn k4_11_an_existing_file_is_never_opened_again() {
    let root = tempfile::tempdir().unwrap();
    let id = RecordingId::from(Uuid::now_v7());
    let path = root.path().join(format!("{id}.anrec"));
    std::fs::write(&path, b"earlier recording").unwrap();
    let refused = create_file(root.path(), id, b"new header").unwrap_err();
    assert_eq!(refused.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&path).unwrap(), b"earlier recording");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_4_no_file_is_made_when_the_row_cannot_be_written(pool: PgPool) {
    let mut world = World::new(&pool, 1 << 30).await;
    world.origin.session = SessionId::from(Uuid::now_v7());
    let refused = begin(&world.serving, world.origin.clone(), PLENTY).await;
    assert!(matches!(refused, Err(Error::Database(_))), "no session row");
    let files = std::fs::read_dir(world.root.path()).unwrap().count();
    assert_eq!(files, 0, "the row comes before the file");
}

#[test]
fn k3_a_recording_key_is_sealed_for_its_recording() {
    let id = RecordingId::from(Uuid::nil());
    assert_eq!(
        key_context(id),
        "sessions.recordings:00000000-0000-0000-0000-000000000000"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn g_4_a_start_dropped_after_its_row_still_ends_the_row(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let session = world.origin.session;
    let mut starting = Box::pin(begin(&world.serving, world.origin.clone(), PLENTY));
    // Polls the start by hand and looks for the row between polls, so the start is dropped
    // with its row written, before it returns.
    let mut id = None;
    eventually("the row is written", async || {
        let polled = std::future::poll_fn(|cx| Poll::Ready(starting.as_mut().poll(cx))).await;
        if let Poll::Ready(started) = polled {
            // The start won the race; its recorder, dropped unfinished, must end the row too.
            id = Some(started.unwrap().id());
            return true;
        }
        id = sqlx::query_scalar("select id from sessions.recordings where session_id = $1")
            .bind(session)
            .fetch_optional(&pool)
            .await
            .unwrap();
        id.is_some()
    })
    .await;
    drop(starting);
    let id = id.unwrap();
    eventually("the row is ended", async || {
        world.stored(id).await.ended_at.is_some()
    })
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_full_part_is_written_without_waiting(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let recorder = world.start(EXACT).await;
    let path = world.path(recorder.id());
    tokio::time::pause();
    recorder.push(&vec![b'x'; MAX_PART + 5]);
    let one_part = HEADER_LEN as u64 + MAX_PART as u64 + FRAMING;
    until(|| length(&path) == one_part).await;
    tokio::time::resume();
    finish_and_check(&world, recorder, MAX_PART + 5).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_the_open_part_is_written_5_seconds_after_its_first_byte(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let recorder = world.start(PLENTY).await;
    let path = world.path(recorder.id());
    tokio::time::pause();
    recorder.push(b"3.nop;");
    tokio::time::advance(FLUSH_AFTER.saturating_sub(Duration::from_millis(1))).await;
    settle().await;
    assert_eq!(length(&path), HEADER_LEN as u64, "not yet");
    // The timer wheel rounds a deadline up to its next millisecond.
    tokio::time::advance(Duration::from_millis(2)).await;
    until(|| length(&path) == HEADER_LEN as u64 + 6 + FRAMING).await;
    tokio::time::resume();
    finish_and_check(&world, recorder, 6).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_a_dropped_recorder_still_ends_its_recording(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    recorder.push(b"3.nop;");
    drop(recorder);
    for _ in 0..200 {
        if world.stored(id).await.ended_at.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.as_slice(), ended), (&b"3.nop;"[..], Ended::Done));
}

/// Finishes `recorder` and checks the file holds `size` bytes of plaintext and its last part.
async fn finish_and_check(world: &World, recorder: Recorder, size: usize) {
    let id = recorder.id();
    recorder.finish().await;
    let (plaintext, ended) = world.read_back(id).await;
    assert_eq!((plaintext.len(), ended), (size, Ended::Done));
    assert_eq!(world.stored(id).await.size_bytes, size as u64);
}
