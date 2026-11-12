//! Operating system detection: `uname -s` answers read as hostile text, and rounds in which a
//! device at Other is detected, then collected from the next round, while an unknown answer
//! and an operator's choice are left alone (DEV-025, MET-005).

use proptest::prelude::*;
use sqlx::PgPool;

use super::bench::{DOCKER_HOST, LINUX_GPU_LINES, LINUX_LINES, bench, events, execute};
use crate::{devices::OperatingSystem, metrics::collector::uname::operating_system};

#[test]
fn dev_025_linux_and_darwin_are_read() {
    assert_eq!(operating_system(b"Linux"), Some(OperatingSystem::Linux));
    assert_eq!(operating_system(b"Darwin"), Some(OperatingSystem::Macos));
    assert_eq!(
        operating_system(b"Linux\n"),
        Some(OperatingSystem::Linux),
        "uname's newline"
    );
    assert_eq!(
        operating_system(b" \tDarwin\r\n"),
        Some(OperatingSystem::Macos),
        "white space around it"
    );
}

#[test]
fn dev_025_any_other_answer_is_no_operating_system() {
    for other in [
        &b"linux\n"[..],
        b"LINUX",
        b"darwin",
        b"FreeBSD\n",
        b"OpenBSD",
        b"MINGW64_NT-10.0",
        b"Linux x86_64",
        b"Linux\nDarwin\n",
        b"Lin ux",
        b"\xffLinux",
        b"",
        b"\n",
    ] {
        assert_eq!(
            operating_system(other),
            None,
            "{:?}",
            String::from_utf8_lossy(other)
        );
    }
}

proptest! {
    /// Whatever a hostile device answers, the result is nothing, Linux or macOS.
    #[test]
    fn dev_025_any_answer_gives_linux_macos_or_nothing(
        answer in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let found = operating_system(&answer);
        prop_assert!(matches!(
            found,
            None | Some(OperatingSystem::Linux | OperatingSystem::Macos)
        ));
    }
}

/// The metrics events about `docker-host`; `pve-01`, also on SSH metrics in the bench, fails
/// every round and has its own.
async fn docker_host_events(pool: &PgPool) -> usize {
    events(pool)
        .await
        .iter()
        .filter(|(_, name, _)| name == "docker-host")
        .count()
}

async fn stored_os(pool: &PgPool) -> String {
    sqlx::query_scalar("select os from devices.devices where id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn samples(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from metrics.samples")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn set_os(pool: &PgPool, os: &str) {
    let sql = format!("update devices.devices set os = '{os}' where id = $1::text::uuid");
    execute(pool, &sql, DOCKER_HOST).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn dev_025_a_device_at_other_is_detected_then_collected_next_round(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    set_os(&pool, "other").await;
    bench.server.answer("uname -s", b"Linux\n");

    bench.collector.round().await;
    assert_eq!(bench.server.heard(), ["uname -s"], "only the detection");
    assert_eq!(stored_os(&pool).await, "linux");
    assert_eq!(samples(&pool).await, 0, "collected from the next round");

    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(
        bench.server.heard(),
        [&["uname -s"][..], &LINUX_LINES[..], &LINUX_GPU_LINES[..]].concat(),
        "detected once, then collected as Linux"
    );
    assert!(samples(&pool).await > 0);
    assert_eq!(docker_host_events(&pool).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn dev_025_an_unknown_answer_leaves_other_and_raises_no_event(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    set_os(&pool, "other").await;
    bench.server.answer("uname -s", b"FreeBSD\n");

    for _ in 0..2 {
        bench.collector.round().await;
        bench.clock.advance(30);
    }
    assert_eq!(
        bench.server.heard(),
        ["uname -s", "uname -s"],
        "asked each round"
    );
    assert_eq!(stored_os(&pool).await, "other");
    assert_eq!(samples(&pool).await, 0);
    assert_eq!(docker_host_events(&pool).await, 0, "no device event");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn dev_025_a_failed_read_leaves_other_and_raises_no_event(pool: PgPool) {
    let mut bench = bench(&pool, false).await;
    set_os(&pool, "other").await;
    bench.server.answer("uname -s", b"Linux\n");

    for _ in 0..2 {
        bench.collector.round().await;
        bench.clock.advance(30);
    }
    assert_eq!(stored_os(&pool).await, "other", "the link never opened");
    assert_eq!(docker_host_events(&pool).await, 0, "no device event");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn dev_025_an_operating_system_the_operator_chose_is_never_asked(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    set_os(&pool, "macos").await;
    bench.server.answer("uname -s", b"Linux\n");

    bench.collector.round().await;
    assert!(
        !bench.server.heard().iter().any(|line| line == "uname -s"),
        "{:?}",
        bench.server.heard()
    );
    assert_eq!(stored_os(&pool).await, "macos");
}
