//! The reader: one read through a fake proxy, the picture it publishes and the warnings it
//! writes (VPN-008, VPN-010). No paused clock: real sockets and a real pool.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    fixture, now,
    proxy::{self, Reply},
};
use crate::{
    clock::FixedClock,
    events::Bus,
    vpn::{
        Feed, Reading, RouteState, VpnPath,
        reader::{read_once, spawn_reader},
    },
};

fn reading(pool: &PgPool, at: DateTime<Utc>) -> Reading {
    Reading {
        pool: pool.clone(),
        bus: Bus::new(16),
        clock: Arc::new(FixedClock(at)),
        feed: Feed::new(),
    }
}

async fn answering(name: &'static str) -> proxy::Proxy {
    let body = fixture(name);
    proxy::start(move || Reply::json(200, &body)).await
}

/// The `vpn` events whose message starts with `prefix`.
async fn events_like(pool: &PgPool, prefix: &str) -> i64 {
    sqlx::query_scalar(
        "select count(*) from events.event_log where source = 'vpn' and message like $1 || '%'",
    )
    .bind(prefix)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_without_a_proxy_no_reader_starts(pool: PgPool) {
    let reading = reading(&pool, now());
    let feed = reading.feed.clone();

    assert!(spawn_reader(None, reading).is_none());
    assert!(feed.latest().is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_a_read_publishes_a_picture(pool: PgPool) {
    let proxy = answering("status-active.json").await;
    let reading = reading(&pool, now());
    let mut failed = false;

    read_once(&proxy.address(), &reading, &mut failed).await;

    let picture = reading.feed.latest().unwrap();
    assert!(picture.fresh && !failed);
    assert_eq!(picture.read_at, now());
    let router = picture
        .nodes
        .iter()
        .find(|n| n.name == "router-home")
        .unwrap();
    assert_eq!(router.path, VpnPath::Direct);
    assert_eq!(picture.sites.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_a_failed_read_keeps_the_picture_and_marks_it_not_fresh(pool: PgPool) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let body = fixture("status-active.json");
    let proxy = proxy::start(move || match counted.fetch_add(1, Ordering::SeqCst) {
        0 => Reply::json(200, &body),
        _ => Reply::json(500, "{}"),
    })
    .await;
    let first = reading(&pool, now());
    let mut failed = false;
    read_once(&proxy.address(), &first, &mut failed).await;
    let later = Reading {
        clock: Arc::new(FixedClock(now() + chrono::Duration::minutes(1))),
        ..first
    };

    read_once(&proxy.address(), &later, &mut failed).await;

    let picture = later.feed.latest().unwrap();
    assert!(failed);
    assert!(!picture.fresh);
    assert_eq!(picture.read_at, now());
    assert_eq!(picture.nodes.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_a_refused_or_unreadable_read_publishes_nothing(pool: PgPool) {
    for reply in [
        Reply::json(403, "{}"),
        Reply::json(200, "{\"Self\": 1}"),
        Reply::json(200, &"x".repeat(1024 * 1024 + 1)),
    ] {
        let proxy = proxy::start(move || reply.clone()).await;
        let reading = reading(&pool, now());
        let mut failed = false;

        read_once(&proxy.address(), &reading, &mut failed).await;

        assert!(failed);
        assert!(reading.feed.latest().is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_the_reader_asks_only_for_status(pool: PgPool) {
    let proxy = answering("status-idle.json").await;
    let reading = reading(&pool, now());
    let mut failed = false;

    read_once(&proxy.address(), &reading, &mut failed).await;
    read_once(&proxy.address(), &reading, &mut failed).await;

    let seen = proxy.seen();
    assert_eq!(seen.len(), 2);
    assert!(
        seen.iter()
            .all(|head| head.starts_with("GET /localapi/v0/status?peers=true ")),
        "{seen:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_warning_is_written_once_across_a_restart(pool: PgPool) {
    let proxy = answering("status-idle.json").await;
    let mut failed = false;

    read_once(&proxy.address(), &reading(&pool, now()), &mut failed).await;
    read_once(&proxy.address(), &reading(&pool, now()), &mut failed).await;

    assert_eq!(
        events_like(&pool, "VPN key of mac expires in 20 days").await,
        1
    );
    assert_eq!(
        events_like(&pool, "VPN node mac (100.64.0.2) is not in Anima").await,
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_29_day_key_then_a_6_day_key_gives_two_events(pool: PgPool) {
    let proxy = answering("status-idle.json").await;
    let expiry = now() + chrono::Duration::days(20);
    let mut failed = false;

    for days_left in [29, 6] {
        let at = expiry - chrono::Duration::days(days_left);
        read_once(&proxy.address(), &reading(&pool, at), &mut failed).await;
    }

    assert_eq!(
        events_like(&pool, "VPN key of mac expires in 29 days").await,
        1
    );
    assert_eq!(
        events_like(&pool, "VPN key of mac expires in 6 days").await,
        1
    );
    assert_eq!(events_like(&pool, "VPN key of mac").await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_009_the_reader_reads_sites_devices_and_ping_checks(pool: PgPool) {
    sqlx::query(
        "insert into devices.sites (id, name, subnet) values \
         ('0190a000-0000-7000-8001-000000000001', 'home', '10.10.0.0/24')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let proxy = answering("status-idle.json").await;
    let reading = reading(&pool, now());
    let mut failed = false;

    read_once(&proxy.address(), &reading, &mut failed).await;

    let picture = reading.feed.latest().unwrap();
    assert_eq!(picture.sites.len(), 1);
    assert_eq!(picture.sites[0].state, RouteState::Serving);
    assert!(!picture.sites[0].has_ping_check);
    assert_eq!(events_like(&pool, "VPN router").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_the_loop_publishes_without_a_tick_wait(pool: PgPool) {
    let proxy = answering("status-idle.json").await;
    let reading = reading(&pool, now());
    let mut receiver = reading.feed.subscribe();

    let task = spawn_reader(Some(proxy.address()), reading).unwrap();

    let published =
        tokio::time::timeout(Duration::from_secs(5), receiver.wait_for(Option::is_some));
    assert!(published.await.is_ok());
    task.abort();
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_008_a_second_failed_read_does_not_wake_a_subscriber(pool: PgPool) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let body = fixture("status-idle.json");
    let proxy = proxy::start(move || match counted.fetch_add(1, Ordering::SeqCst) {
        0 => Reply::json(200, &body),
        _ => Reply::json(500, "{}"),
    })
    .await;
    let reading = reading(&pool, now());
    let mut failed = false;
    read_once(&proxy.address(), &reading, &mut failed).await;
    read_once(&proxy.address(), &reading, &mut failed).await;
    let mut receiver = reading.feed.subscribe();
    receiver.mark_unchanged();

    read_once(&proxy.address(), &reading, &mut failed).await;

    assert!(failed);
    assert!(!reading.feed.latest().unwrap().fresh);
    assert!(!receiver.has_changed().unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_a_failed_event_leaves_no_mark_so_the_next_read_writes_it(pool: PgPool) {
    let proxy = answering("status-idle.json").await;
    let mut failed = false;
    sqlx::query("alter table events.event_log add constraint no_vpn check (source <> 'vpn')")
        .execute(&pool)
        .await
        .unwrap();

    read_once(&proxy.address(), &reading(&pool, now()), &mut failed).await;

    assert!(failed);
    let marks: i64 = sqlx::query_scalar("select count(*) from vpn.warnings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(marks, 0);
    sqlx::query("alter table events.event_log drop constraint no_vpn")
        .execute(&pool)
        .await
        .unwrap();

    read_once(&proxy.address(), &reading(&pool, now()), &mut failed).await;

    assert!(!failed);
    assert_eq!(events_like(&pool, "VPN node mac").await, 1);
}
