//! The sweep against the database: the time rules and the place rules (REM-002, REM-006).

use chrono_tz::Tz;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::{
    engine::{
        locations::{self, NewSample, Origin, Seal},
        sealing_keys::SealingKeys,
    },
    ids::PlaceId,
};

const HOME: u128 = 0x0190_a000_0000_7000_9200_0000_0000_0001;

fn seal() -> (Reading, tempfile::TempDir) {
    let folder = tempfile::tempdir().unwrap();
    let keys = SealingKeys::open(Some(folder.path()), b"credential").unwrap();
    let reading = Reading {
        seal: Seal::new(keys),
        extensions: Vec::new(),
    };
    (reading, folder)
}

/// A top-level place of 100 m around 52.0, 4.0.
async fn add_home(pool: &PgPool) -> PlaceId {
    sqlx::query(
        "insert into places.place \
         (id, name, latitude, longitude, radius_m, category, icon, created_at, updated_at) \
         values ($1, 'Home', 52.0, 4.0, 100, 'home', 'house', now(), now())",
    )
    .bind(Uuid::from_u128(HOME))
    .execute(pool)
    .await
    .unwrap();
    PlaceId::from(Uuid::from_u128(HOME))
}

/// Stores one fix that the phone made at `observed` and that the hub got at `received`.
async fn fix(pool: &PgPool, seal: &Reading, at_home: bool, (observed, received): (&str, &str)) {
    let sample = NewSample {
        key: format!("{observed}:{received}"),
        observed_at: Some(at(observed)),
        position: locations::Position {
            latitude: Some(if at_home { 52.0 } else { 52.01 }),
            longitude: Some(4.0),
            accuracy_m: Some(10.0),
            ..locations::Position::default()
        },
    };
    let origin = Origin {
        provider: locations::Provider::OwnTracks,
        account: "phone".to_owned(),
    };
    let mut conn = pool.acquire().await.unwrap();
    locations::store_samples(&mut conn, &seal.seal, (&origin, &[sample]), at(received))
        .await
        .unwrap();
}

async fn subjects(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as("select subject, class from reminders.firing order by due_at, subject")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_002_the_sweep_fires_a_time_rule_one_time_for_each_of_its_instants(pool: PgPool) {
    let (seal, _keys) = seal();
    add(&pool, rule(daily((9, 0), Vec::new(), Tz::UTC))).await;
    let early = sweep(&pool, &seal, at("2026-11-19T08:59:30Z"))
        .await
        .unwrap();
    assert_eq!(
        early.firings, 1,
        "the instant of the day before was missed: one time"
    );
    for now in [
        "2026-11-19T09:00:30Z",
        "2026-11-19T09:01:30Z",
        "2026-11-19T09:02:30Z",
    ] {
        sweep(&pool, &seal, at(now)).await.unwrap();
    }
    let fired: Vec<String> = subjects(&pool).await.into_iter().map(|(s, _)| s).collect();
    assert_eq!(
        fired,
        ["time:2026-11-18T09:00:00Z", "time:2026-11-19T09:00:00Z"]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_006_the_sweep_fires_an_arrival_one_time_and_not_for_a_late_import(pool: PgPool) {
    let (seal, _keys) = seal();
    let home = add_home(&pool).await;
    add(
        &pool,
        rule(Trigger::Place {
            place: home,
            on: Crossing::Arrive,
        }),
    )
    .await;

    fix(
        &pool,
        &seal,
        false,
        ("2026-11-19T12:00:00Z", "2026-11-19T12:00:02Z"),
    )
    .await;
    let away = sweep(&pool, &seal, at("2026-11-19T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(
        (away.place_changes, away.firings),
        (0, 0),
        "a place with no state is outside"
    );

    fix(
        &pool,
        &seal,
        true,
        ("2026-11-19T12:05:00Z", "2026-11-19T12:05:02Z"),
    )
    .await;
    let arrived = sweep(&pool, &seal, at("2026-11-19T12:05:30Z"))
        .await
        .unwrap();
    assert_eq!((arrived.place_changes, arrived.firings), (1, 1));
    assert_eq!(
        subjects(&pool).await[0].1,
        "B",
        "where the operator is, is Class B"
    );

    // A second fix at home is no arrival, and the same sweep again reads nothing.
    fix(
        &pool,
        &seal,
        true,
        ("2026-11-19T12:06:00Z", "2026-11-19T12:06:02Z"),
    )
    .await;
    let still = sweep(&pool, &seal, at("2026-11-19T12:06:30Z"))
        .await
        .unwrap();
    assert_eq!((still.place_changes, still.firings), (0, 0));
    let again = sweep(&pool, &seal, at("2026-11-19T12:06:40Z"))
        .await
        .unwrap();
    assert_eq!(again, Swept::default());

    // The operator leaves. Then an old fix from before the departure arrives late: it changes
    // no state, so the next arrival is still an arrival.
    fix(
        &pool,
        &seal,
        false,
        ("2026-11-19T14:00:00Z", "2026-11-19T14:00:02Z"),
    )
    .await;
    sweep(&pool, &seal, at("2026-11-19T14:00:30Z"))
        .await
        .unwrap();
    fix(
        &pool,
        &seal,
        true,
        ("2026-11-19T13:00:00Z", "2026-11-19T14:10:00Z"),
    )
    .await;
    let late = sweep(&pool, &seal, at("2026-11-19T14:10:30Z"))
        .await
        .unwrap();
    assert_eq!((late.place_changes, late.firings), (0, 0));
    assert_eq!(subjects(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_006_an_arrival_that_the_hub_reads_late_changes_the_state_and_fires_nothing(
    pool: PgPool,
) {
    let (seal, _keys) = seal();
    let home = add_home(&pool).await;
    add(
        &pool,
        rule(Trigger::Place {
            place: home,
            on: Crossing::Arrive,
        }),
    )
    .await;
    // The phone was offline: the fix of 12:00 reaches the hub at 12:30.
    fix(
        &pool,
        &seal,
        true,
        ("2026-11-19T12:00:00Z", "2026-11-19T12:30:00Z"),
    )
    .await;
    let read = sweep(&pool, &seal, at("2026-11-19T12:30:30Z"))
        .await
        .unwrap();
    assert_eq!(
        (read.place_changes, read.firings),
        (1, 0),
        "older than 10 minutes"
    );
    let inside: bool = sqlx::query_scalar("select inside from reminders.place_state")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(inside);
}
