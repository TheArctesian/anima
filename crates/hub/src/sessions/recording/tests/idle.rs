//! An idle writer wakes (K4 §11): when a push fills a part, and when a push stops the
//! recording, the writer is woken at once, with no timer and no `finish` to wake it.

use std::time::Duration;

use sqlx::PgPool;

use super::{FRAMING, PLENTY, World, eventually, length};
use crate::sessions::recording::{
    format::{HEADER_LEN, MAX_PART},
    model::StopReason,
};

/// Lets the writer task run until it waits for work: nothing is queued, so it waits with no
/// deadline and only a wake-up moves it.
async fn idle() {
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_an_idle_writer_wakes_for_a_full_part(pool: PgPool) {
    let world = World::new(&pool, 1 << 30).await;
    let recorder = world.start(PLENTY).await;
    let path = world.path(recorder.id());
    idle().await;
    recorder.push(&vec![b'x'; MAX_PART]);
    let one_part = HEADER_LEN as u64 + MAX_PART as u64 + FRAMING;
    eventually("one part on the disk", async || length(&path) == one_part).await;
    recorder.finish().await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_11_an_idle_writer_wakes_for_a_stop(pool: PgPool) {
    let world = World::new(&pool, 10).await;
    let recorder = world.start(PLENTY).await;
    let id = recorder.id();
    idle().await;
    recorder.push(b"0123456789ab");
    eventually("the row is ended before finish", async || {
        world.stored(id).await.ended_at.is_some()
    })
    .await;
    assert_eq!(world.stored(id).await.stopped, Some(StopReason::SizeLimit));
    recorder.finish().await;
}
