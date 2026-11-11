//! The VPN reader through the hub's public face: what reaches the process log (VPN-008,
//! LOG-007).

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anima_hub::{
    clock::FixedClock,
    events::{Bus, process_log},
    vpn::{Feed, Reading, spawn_reader},
};
use sqlx::PgPool;

use super::proxy::{self, Reply};
use crate::{hub::start, logs::Capture};

/// The hub's status as the fake proxy answers it: three nodes, `hub`, `mac`, `router-home`.
pub const STATUS: &str = include_str!("../../../src/vpn/tests/fixtures/status-active.json");

pub fn reading(pool: &PgPool, feed: &Feed) -> Reading {
    Reading {
        pool: pool.clone(),
        bus: Bus::new(16),
        clock: Arc::new(FixedClock(start())),
        feed: feed.clone(),
    }
}

/// Waits until the feed holds a picture that `accepts` accepts.
async fn published(feed: &Feed, accepts: impl Fn(&anima_hub::vpn::Picture) -> bool) {
    let mut receiver = feed.subscribe();
    let waited = tokio::time::timeout(
        Duration::from_secs(5),
        receiver.wait_for(|latest| latest.as_deref().is_some_and(&accepts)),
    );
    assert!(waited.await.is_ok(), "no picture in time");
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_no_log_line_holds_status_text(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("info", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    // The failed answer holds the status text too, so a logged body would show.
    let proxy = proxy::start(move || match counted.fetch_add(1, Ordering::SeqCst) {
        0 => Reply::json(200, STATUS),
        _ => Reply::json(500, STATUS),
    })
    .await;
    let feed = Feed::new();

    let first = spawn_reader(Some(proxy.address()), reading(&pool, &feed)).unwrap();
    published(&feed, |picture| picture.fresh).await;
    let second = spawn_reader(Some(proxy.address()), reading(&pool, &feed)).unwrap();
    published(&feed, |picture| !picture.fresh).await;
    first.abort();
    second.abort();

    let asked = proxy.seen();
    assert!(
        asked
            .iter()
            .all(|head| head.starts_with("GET /localapi/v0/status?peers=true ")),
        "{asked:?}"
    );
    let text = capture.text();
    assert!(text.contains("VPN status read failed"), "{text}");
    for leak in [
        "router-home",
        "HostName",
        "nodekey:",
        "100.64.",
        "192.0.2.",
        "tag:",
    ] {
        assert!(!text.contains(leak), "{leak:?} in {text}");
    }
}
