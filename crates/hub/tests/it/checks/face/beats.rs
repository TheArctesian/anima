//! `record_beat`: the heartbeat, its minute, hour and day rows and the check's state, all in
//! the caller's transaction (CHK-003, DB-008).

use std::net::IpAddr;

use anima_hub::{
    checks::{self, Error, Missing, Recorded},
    ids::CheckId,
    status::Status,
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use sqlx::{Connection as _, PgPool};

use super::{PVE_01, beat, count, create, database_now, icmp, new_check, record};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const UPTIME_TABLES: [&str; 3] = ["uptime_minute", "uptime_hour", "uptime_day"];
const HEARTBEATS: &str = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";

/// (bucket, up, down, degraded, unknown, response count, average) of each aggregate table.
type Aggregate = (DateTime<Utc>, i32, i32, i32, i32, i32, Option<f32>);

async fn aggregates(pool: &PgPool, id: CheckId) -> Vec<Vec<Aggregate>> {
    let mut all = Vec::new();
    for table in UPTIME_TABLES {
        let rows: Vec<Aggregate> = sqlx::query_as(&format!(
            "SELECT bucket, up_count, down_count, degraded_count, unknown_count, \
                    response_count, avg_response_ms \
             FROM checks.{table} WHERE check_id = $1 ORDER BY bucket"
        ))
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap();
        all.push(rows);
    }
    all
}

fn truncated(at: DateTime<Utc>, step: TimeDelta) -> DateTime<Utc> {
    at.duration_trunc(step).unwrap()
}

/// (status, failures, next due, last beat, resolved address, certificate expiry).
type State = (
    String,
    i32,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<IpAddr>,
    Option<DateTime<Utc>>,
);

async fn state(pool: &PgPool, id: CheckId) -> State {
    sqlx::query_as(
        "SELECT status, failures, next_due_at, last_beat_at, resolved_ip, cert_not_after \
         FROM checks.checks WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

fixture_test! {
    async fn db_008_a_beat_writes_the_heartbeat_three_aggregate_rows_and_the_state(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Beat", icmp())).await;
        let at = database_now(&pool).await;
        let expiry = at + TimeDelta::days(40);
        let mut up = beat(at, Status::Up, Some(40));
        up.state.failures = 0;
        up.resolved_ip = Some("10.10.0.2".parse().unwrap());
        up.cert_not_after = Some(expiry);
        record(&pool, id, &up).await;

        let heartbeat: (DateTime<Utc>, String, Option<i32>) = sqlx::query_as(
            "SELECT at, status, response_time_ms FROM checks.heartbeats WHERE check_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(heartbeat, (at, "up".to_owned(), Some(40)));

        let steps = [TimeDelta::minutes(1), TimeDelta::hours(1), TimeDelta::days(1)];
        for (rows, step) in aggregates(&pool, id).await.into_iter().zip(steps) {
            assert_eq!(rows, [(truncated(at, step), 1, 0, 0, 0, 1, Some(40.0))]);
        }
        assert_eq!(
            state(&pool, id).await,
            (
                "up".to_owned(),
                0,
                Some(at + TimeDelta::seconds(60)),
                Some(at),
                up.resolved_ip,
                Some(expiry),
            )
        );
    }

    async fn db_008_aggregates_count_each_status_and_average_the_answered_beats(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Average", icmp())).await;
        let base = truncated(database_now(&pool).await, TimeDelta::minutes(1));
        let beats = [
            beat(base, Status::Up, Some(10)),
            beat(base + TimeDelta::seconds(1), Status::Up, Some(30)),
            beat(base + TimeDelta::seconds(2), Status::Down, None),
            beat(base + TimeDelta::seconds(3), Status::Unknown, None),
            beat(base + TimeDelta::seconds(4), Status::Degraded, Some(80)),
        ];
        for each in &beats {
            record(&pool, id, each).await;
        }
        for rows in aggregates(&pool, id).await {
            assert_eq!(rows.len(), 1);
            let (_, up, down, degraded, unknown, responses, average) = rows[0];
            assert_eq!((up, down, degraded, unknown, responses), (2, 1, 1, 1, 3));
            assert_eq!(average, Some(40.0));
        }
        let heartbeats = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";
        assert_eq!(count(&pool, heartbeats, id).await, 5);
    }

    async fn chk_003_the_message_is_stripped_of_control_characters_and_capped(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Message", icmp())).await;
        let mut down = beat(database_now(&pool).await, Status::Down, None);
        down.message = Some(format!("bad\r\n\u{7}{}", "x".repeat(600)));
        record(&pool, id, &down).await;
        let message: String =
            sqlx::query_scalar("SELECT message FROM checks.heartbeats WHERE check_id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(message.starts_with("badxxx"), "{message:?}");
        assert_eq!(message.chars().count(), 512);
    }

    async fn db_008_the_address_and_expiry_stay_when_a_later_probe_has_none(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Sticky", icmp())).await;
        let at = database_now(&pool).await;
        let mut first = beat(at, Status::Up, Some(5));
        first.resolved_ip = Some("10.10.0.2".parse().unwrap());
        first.cert_not_after = Some(at + TimeDelta::days(3));
        record(&pool, id, &first).await;
        record(&pool, id, &beat(at + TimeDelta::seconds(1), Status::Down, None)).await;
        let (status, _, _, last, ip, expiry) = state(&pool, id).await;
        assert_eq!(status, "down");
        assert_eq!(last, Some(at + TimeDelta::seconds(1)));
        assert_eq!((ip, expiry), (first.resolved_ip, first.cert_not_after));
    }

    async fn db_008_an_older_beat_after_a_newer_one_changes_nothing(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Ordered", icmp())).await;
        let at = database_now(&pool).await;
        record(&pool, id, &beat(at, Status::Up, Some(5))).await;
        let before = state(&pool, id).await;
        let mut conn = pool.acquire().await.unwrap();
        let late = beat(at - TimeDelta::seconds(5), Status::Down, None);
        assert_eq!(checks::record_beat(&mut conn, id, &late).await.unwrap(), Recorded::Skipped);
        assert_eq!(state(&pool, id).await, before);
        assert_eq!(count(&pool, HEARTBEATS, id).await, 1);
        assert_eq!(aggregates(&pool, id).await[0][0].1, 1);
    }

    async fn chk_005_a_beat_for_a_paused_check_changes_nothing(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Paused", icmp())).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        let before = state(&pool, id).await;
        let down = beat(database_now(&pool).await, Status::Down, None);
        assert_eq!(checks::record_beat(&mut conn, id, &down).await.unwrap(), Recorded::Skipped);
        assert_eq!(state(&pool, id).await, before);
        assert_eq!(count(&pool, HEARTBEATS, id).await, 0);
        assert!(aggregates(&pool, id).await.iter().all(Vec::is_empty));
    }

    async fn db_008_a_failed_heartbeat_step_rolls_back_only_its_savepoint(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Savepoint", icmp())).await;
        let outside = database_now(&pool).await - TimeDelta::days(40);
        let mut conn = pool.acquire().await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        let refused = checks::record_beat(&mut tx, id, &beat(outside, Status::Down, None)).await;
        assert!(matches!(refused, Err(Error::Database(_))), "{refused:?}");
        let name: String = sqlx::query_scalar("SELECT name FROM checks.checks WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(name, "Savepoint");
        tx.commit().await.unwrap();
        let (status, failures, due, last, _, _) = state(&pool, id).await;
        assert_eq!((status.as_str(), failures, due, last), ("unknown", 0, None, None));
        assert!(aggregates(&pool, id).await.iter().all(Vec::is_empty));
        assert_eq!(count(&pool, HEARTBEATS, id).await, 0);
    }

    async fn chk_003_a_beat_for_a_missing_check_writes_nothing(pool: PgPool) {
        let gone: CheckId = "0190a000-0000-7000-8003-0000000000ff".parse().unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let error = checks::record_beat(&mut conn, gone, &beat(database_now(&pool).await, Status::Up, Some(1)))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::NotFound(Missing::Check(id)) if id == gone), "{error:?}");
        let heartbeats = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";
        assert_eq!(count(&pool, heartbeats, gone).await, 0);
    }

    async fn db_008_a_beat_in_a_rolled_back_transaction_leaves_nothing(pool: PgPool) {
        let id = create(&pool, new_check(PVE_01, "Rolled back", icmp())).await;
        let at = database_now(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        let recorded = checks::record_beat(&mut tx, id, &beat(at, Status::Down, None)).await;
        assert_eq!(recorded.unwrap(), Recorded::Written);
        tx.rollback().await.unwrap();
        let heartbeats = "SELECT count(*) FROM checks.heartbeats WHERE check_id = $1";
        assert_eq!(count(&pool, heartbeats, id).await, 0);
        assert!(aggregates(&pool, id).await.iter().all(Vec::is_empty));
        assert_eq!(state(&pool, id).await.0, "unknown");
    }
}
