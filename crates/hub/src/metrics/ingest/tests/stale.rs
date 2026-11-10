//! MET-018, R-098 a: an agent is stale after more than 30 seconds with no accepted batch, with
//! one event when it goes stale and one when it sends again; a refused batch does not count, a
//! batch of only unknown series does, an agent that never pushed is never marked, a device
//! switched away from the agent is marked without an event, and one device that cannot be
//! marked does not keep the others from being marked.

use axum::http::StatusCode;
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{database_now, enrolled_pve, push, pve};
use crate::{events::Bus, metrics::ingest::stale::mark_stale};

/// A batch of pve-01 with one point at `at` of `kind`.
fn batch(at: DateTime<Utc>, kind: &str) -> Value {
    let at = at.duration_trunc(TimeDelta::seconds(10)).unwrap();
    json!({
        "device": pve().to_string(),
        "series": [{ "kind": kind }],
        "points": [{ "at": at.format("%Y-%m-%dT%H:%M:%SZ").to_string(), "mean": [5.0] }],
    })
}

/// The metrics events of pve-01, oldest first.
async fn events(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE source = 'metrics' AND device_id = $1 \
         AND message LIKE 'The agent on pve-01 s%' ORDER BY at, id",
    )
    .bind(pve())
    .fetch_all(pool)
    .await
    .unwrap()
}

const STALE: &str = "The agent on pve-01 sent no metrics for 30 seconds; they are stale.";
const FRESH: &str = "The agent on pve-01 sends metrics again.";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_018_stale_after_more_than_30_seconds_then_fresh_with_one_event_each_way(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let bus = Bus::new(16);
    assert_eq!(
        push(&pool, now, &token, &batch(now, "cpu_percent")).await,
        StatusCode::NO_CONTENT
    );

    let thirty = now + TimeDelta::seconds(30);
    assert_eq!(
        mark_stale(&pool, &bus, thirty).await.unwrap(),
        0,
        "30 s is not stale yet"
    );
    let later = thirty + TimeDelta::seconds(1);
    assert_eq!(mark_stale(&pool, &bus, later).await.unwrap(), 1);
    assert_eq!(
        mark_stale(&pool, &bus, later + TimeDelta::seconds(5))
            .await
            .unwrap(),
        0
    );
    assert_eq!(events(&pool).await, [STALE]);

    let back = later + TimeDelta::seconds(9);
    assert_eq!(
        push(&pool, back, &token, &batch(back, "cpu_percent")).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        push(&pool, back, &token, &batch(back, "memory_percent")).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(events(&pool).await, [STALE, FRESH]);
    assert_eq!(
        mark_stale(&pool, &bus, back + TimeDelta::seconds(30))
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn r_098_a_a_refused_batch_keeps_the_timer_and_an_unknown_kind_resets_it(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let bus = Bus::new(16);
    assert_eq!(
        push(&pool, now, &token, &batch(now, "cpu_percent")).await,
        StatusCode::NO_CONTENT
    );

    let refused_at = now + TimeDelta::seconds(20);
    let mut other = batch(refused_at, "cpu_percent");
    other["device"] = json!("0190a000-0000-7000-8002-000000000002");
    assert_eq!(
        push(&pool, refused_at, &token, &other).await,
        StatusCode::FORBIDDEN
    );
    let mut late = batch(refused_at, "cpu_percent");
    late["points"][0]["at"] = json!("2001-01-01T00:00:00Z");
    assert_eq!(
        push(&pool, refused_at, &token, &late).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let stale_at = now + TimeDelta::seconds(31);
    assert_eq!(
        mark_stale(&pool, &bus, stale_at).await.unwrap(),
        1,
        "refusals do not count"
    );

    let unknown_at = stale_at + TimeDelta::seconds(10);
    let unknown = batch(unknown_at, "fan_rpm");
    assert_eq!(
        push(&pool, unknown_at, &token, &unknown).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(events(&pool).await, [STALE, FRESH]);
    let samples: i64 = sqlx::query_scalar("SELECT count(*) FROM metrics.samples")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(samples, 1, "the unknown kind stored nothing");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_018_an_agent_that_never_pushed_is_never_marked(pool: PgPool) {
    let now = database_now(&pool).await;
    enrolled_pve(&pool, now, 1).await;
    let bus = Bus::new(16);
    let much_later = now + TimeDelta::hours(2);
    assert_eq!(mark_stale(&pool, &bus, much_later).await.unwrap(), 0);
    assert_eq!(events(&pool).await, [] as [String; 0]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_018_a_device_switched_away_from_the_agent_is_marked_without_an_event(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let bus = Bus::new(16);
    assert_eq!(
        push(&pool, now, &token, &batch(now, "cpu_percent")).await,
        StatusCode::NO_CONTENT
    );
    sqlx::query("UPDATE devices.devices SET metrics_source = 'ssh' WHERE id = $1")
        .bind(pve())
        .execute(&pool)
        .await
        .unwrap();

    let later = now + TimeDelta::seconds(31);
    assert_eq!(mark_stale(&pool, &bus, later).await.unwrap(), 1);
    assert_eq!(
        mark_stale(&pool, &bus, later).await.unwrap(),
        0,
        "kept out of later rounds"
    );
    assert_eq!(events(&pool).await, [] as [String; 0]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_018_one_device_that_cannot_be_marked_does_not_keep_the_others_fresh(pool: PgPool) {
    let now = database_now(&pool).await;
    let docker = "0190a000-0000-7000-8002-000000000002";
    sqlx::query("UPDATE devices.devices SET metrics_source = 'agent' WHERE id = ANY($1::uuid[])")
        .bind([pve().to_string(), docker.to_owned()])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO metrics.agents (device_id, token_sha256, expires_at, last_batch_at) \
         SELECT d, sha256(d::text::bytea), $2, $3 FROM unnest($1::uuid[]) AS d",
    )
    .bind([pve().to_string(), docker.to_owned()])
    .bind(now + TimeDelta::days(1))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();
    // The device sorted first fails each time it is marked; the other must still be.
    sqlx::raw_sql(&format!(
        "CREATE FUNCTION metrics.refuse_mark() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN RAISE EXCEPTION 'cannot mark'; END $$; \
         CREATE TRIGGER refuse_mark BEFORE UPDATE ON metrics.agents FOR EACH ROW \
         WHEN (NEW.device_id = '{}') EXECUTE FUNCTION metrics.refuse_mark();",
        pve()
    ))
    .execute(&pool)
    .await
    .unwrap();

    let later = now + TimeDelta::seconds(31);
    assert!(mark_stale(&pool, &Bus::new(16), later).await.is_err());
    let stale: Vec<String> =
        sqlx::query_scalar("SELECT device_id::text FROM metrics.agents WHERE stale_at IS NOT NULL")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(stale, [docker]);
}
