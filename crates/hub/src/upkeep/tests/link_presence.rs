//! The check `Connected` of the computers of the link (HMD-047): which devices are
//! computers of the link, what the last sign of one means, and what one pass reports.

use std::{path::PathBuf, sync::Arc};

use anima_hub::{
    clock::FixedClock,
    fleet::{checks, devices, events::Bus, link::Presence},
    ids::{CheckId, DeviceId},
    status::Status,
};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use sqlx::PgPool;

use crate::upkeep::link_presence::{Sign, Watch, computers, sign};

/// `docker-host` and `ollama` of the K6 fixtures.
const LAPTOP: &str = "0190a000-0000-7000-8002-000000000002";
const DESKTOP: &str = "0190a000-0000-7000-8002-000000000003";
const LAPTOP_ADDRESS: &str = "fd41:4e49:4d41:5acc::7";
const DESKTOP_ADDRESS: &str = "fd41:4e49:4d41:5acc::8";

fn seconds(n: i64) -> TimeDelta {
    TimeDelta::seconds(n)
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn listed(address: &str, seen: Option<DateTime<Utc>>) -> Presence {
    Presence {
        address: address.parse().unwrap(),
        seen,
    }
}

#[test]
fn hmd_047_a_computer_is_connected_while_its_last_sign_is_45_seconds_old_or_newer() {
    let now: DateTime<Utc> = "2026-11-18T12:00:00Z".parse().unwrap();
    let long = now - seconds(3600);
    assert_eq!(sign(Some(now), now, long), Sign::Here);
    assert_eq!(sign(Some(now - seconds(45)), now, long), Sign::Here);
    assert_eq!(sign(Some(now - seconds(46)), now, long), Sign::Gone);
    // The link never saw it: an older link program, or a computer that is off.
    assert_eq!(sign(None, now, long), Sign::Unknown);
}

#[test]
fn hmd_047_an_old_sign_is_not_gone_until_the_hub_has_watched_for_45_seconds() {
    let now: DateTime<Utc> = "2026-11-18T12:00:00Z".parse().unwrap();
    let old = Some(now - seconds(3600));
    assert_eq!(sign(old, now, now), Sign::Unknown);
    assert_eq!(sign(old, now, now - seconds(44)), Sign::Unknown);
    assert_eq!(sign(old, now, now - seconds(45)), Sign::Gone);
    // A computer that is connected is Up at once.
    assert_eq!(sign(Some(now - seconds(5)), now, now), Sign::Here);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn hmd_047_a_computer_of_the_link_is_a_device_whose_host_is_a_listed_address(pool: PgPool) {
    // Not the short form that the link writes: the same address.
    set_host(&pool, LAPTOP, "fd41:4e49:4d41:5acc:0:0:0:7").await;
    let devices = devices::list(&pool).await.unwrap();
    let seen = Some(Utc::now());
    let presence = [
        listed(LAPTOP_ADDRESS, seen),
        listed(DESKTOP_ADDRESS, None),
        // An address of the fixtures that is not a link address of this list.
        listed("fd41:4e49:4d41::99", seen),
    ];

    assert_eq!(computers(&devices, &presence), [(device(LAPTOP), seen)]);
    assert_eq!(computers(&devices, &[]), []);
}

async fn set_host(pool: &PgPool, id: &str, host: &str) {
    sqlx::query("update devices.devices set host = $2 where id = $1::uuid")
        .bind(id)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

/// A hub with two computers of the link among its devices, and its watch, started at `at`.
async fn watch_at(pool: &PgPool, at: DateTime<Utc>) -> Watch {
    set_host(pool, LAPTOP, LAPTOP_ADDRESS).await;
    set_host(pool, DESKTOP, DESKTOP_ADDRESS).await;
    let clock = Arc::new(FixedClock(at));
    let shared = (pool.clone(), Bus::new(16));
    Watch::new(PathBuf::from("/nonexistent/control.sock"), shared, clock)
}

async fn now_of(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(seconds(1)).unwrap()
}

async fn connected_check(pool: &PgPool, id: &str) -> Option<CheckId> {
    let own = checks::OwnCheck::Connected;
    checks::own::find(pool, device(id), own).await.unwrap()
}

/// The status and the message of each result of the check, the oldest first.
async fn results(pool: &PgPool, check: CheckId) -> Vec<(String, Option<String>)> {
    sqlx::query_as(
        "select status::text, message from checks.heartbeats where check_id = $1 order by at",
    )
    .bind(check)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn hmd_047_a_connected_computer_gets_the_check_and_is_up_and_one_never_seen_gets_none(
    pool: PgPool,
) {
    let now = now_of(&pool).await;
    let mut watch = watch_at(&pool, now).await;
    let presence = [
        listed(LAPTOP_ADDRESS, Some(now - seconds(10))),
        listed(DESKTOP_ADDRESS, None),
    ];

    watch.pass(&presence, now).await.unwrap();
    watch.pass(&presence, now + seconds(15)).await.unwrap();

    let check = connected_check(&pool, LAPTOP).await.expect("the check");
    let stored = checks::get(&pool, check).await.unwrap();
    assert_eq!(stored.name, "Connected");
    assert!(stored.push_token.is_some(), "a PUSH check");
    let up = ("up".to_owned(), None);
    assert_eq!(results(&pool, check).await, [up.clone(), up]);
    let status = checks::device_status(&pool, device(LAPTOP), now).await;
    assert_eq!(status.unwrap(), Status::Up);
    assert_eq!(connected_check(&pool, DESKTOP).await, None);
    let all = checks::for_device(&pool, device(DESKTOP)).await.unwrap();
    assert_eq!(all.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn hmd_047_a_computer_that_left_the_link_is_reported_down_one_time(pool: PgPool) {
    let now = now_of(&pool).await - seconds(120);
    let mut watch = watch_at(&pool, now).await;
    let here = [listed(LAPTOP_ADDRESS, Some(now))];
    watch.pass(&here, now).await.unwrap();
    let check = connected_check(&pool, LAPTOP).await.expect("the check");
    // Three more retries wait before a failed attempt is Down; none here.
    sqlx::query("update checks.checks set retries = 0 where id = $1")
        .bind(check)
        .execute(&pool)
        .await
        .unwrap();

    // The sign is 45 seconds old: still connected, and nothing new to say but Up.
    watch.pass(&here, now + seconds(45)).await.unwrap();
    watch.pass(&here, now + seconds(60)).await.unwrap();
    watch.pass(&here, now + seconds(75)).await.unwrap();

    let up = ("up".to_owned(), None);
    let down = ("down".to_owned(), Some("Not connected".to_owned()));
    assert_eq!(
        results(&pool, check).await,
        [up.clone(), up.clone(), down.clone()]
    );

    // It connects again, and leaves again: a new report.
    let back = [listed(LAPTOP_ADDRESS, Some(now + seconds(90)))];
    watch.pass(&back, now + seconds(90)).await.unwrap();
    watch.pass(&back, now + seconds(136)).await.unwrap();
    let all = [up.clone(), up.clone(), down.clone(), up, down];
    assert_eq!(results(&pool, check).await, all);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn hmd_047_after_a_start_or_a_sleep_no_computer_is_down_and_each_check_waits_again(
    pool: PgPool,
) {
    let now = now_of(&pool).await - seconds(120);
    let mut watch = watch_at(&pool, now).await;
    let here = [listed(LAPTOP_ADDRESS, Some(now))];
    watch.pass(&here, now).await.unwrap();
    let check = connected_check(&pool, LAPTOP).await.expect("the check");
    let due = || async { checks::get(&pool, check).await.unwrap().state.next_due_at };
    assert_eq!(due().await, Some(now + seconds(60)));

    // The hub slept for 50 seconds; the link's last sign is as old.
    let woke = now + seconds(50);
    watch.begin(woke).await;
    watch.pass(&here, woke).await.unwrap();
    watch.pass(&here, woke + seconds(30)).await.unwrap();

    assert_eq!(due().await, Some(woke + seconds(60)));
    assert_eq!(
        results(&pool, check).await.len(),
        1,
        "no report but the first Up"
    );
    // A check that waits longer already keeps its time.
    watch.begin(woke - seconds(30)).await;
    assert_eq!(due().await, Some(woke + seconds(60)));
}
