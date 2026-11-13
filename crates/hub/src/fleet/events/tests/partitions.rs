//! Day partitions of the event and audit logs (DB-004, D-4).

use sqlx::{PgConnection, PgPool, Transaction};

use serde_json::json;
use sqlx::Postgres;

use super::{day, device_event, new_audit, partition_of};
use crate::fleet::events::{Bus, Error, audit, create_partitions_ahead, record};

const ENSURE: &str = "select events.ensure_day_partition('event_log', $1)";

pub(super) async fn table_exists(conn: &mut PgConnection, name: &str) -> bool {
    sqlx::query_scalar("select to_regclass($1) is not null")
        .bind(name)
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Counts `event_log` rows on a fresh connection that waits at most one second for a lock.
async fn count_with_lock_timeout(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let mut reader = pool.acquire().await.unwrap();
    sqlx::query("set lock_timeout = '1s'")
        .execute(&mut *reader)
        .await
        .unwrap();
    sqlx::query_scalar("select count(*) from events.event_log")
        .fetch_one(&mut *reader)
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_record_creates_a_day_partition_for_any_date(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let bus = Bus::new(4);

    let first = record(&mut conn, &bus, day(2026, 1, 1), device_event("a"));
    let first = first.await.unwrap();
    let same_day = record(&mut conn, &bus, day(2026, 1, 1), device_event("b"));
    let same_day = same_day.await.unwrap();
    let later = record(&mut conn, &bus, day(2031, 6, 30), device_event("c"));
    let later = later.await.unwrap();

    assert_eq!(
        partition_of(&mut conn, first).await,
        "events.event_log_20260101"
    );
    assert_eq!(
        partition_of(&mut conn, same_day).await,
        "events.event_log_20260101"
    );
    assert_eq!(
        partition_of(&mut conn, later).await,
        "events.event_log_20310630"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_create_partitions_ahead_covers_today_and_the_next_days(pool: PgPool) {
    create_partitions_ahead(&pool, day(2030, 12, 31), 1)
        .await
        .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    for table in ["event_log", "audit_log"] {
        for (date, exists) in [("20301231", true), ("20310101", true), ("20310102", false)] {
            let name = format!("events.{table}_{date}");
            assert_eq!(table_exists(&mut conn, &name).await, exists, "{name}");
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_open_writer_on_a_day_created_ahead_does_not_block_readers(pool: PgPool) {
    let bus = Bus::new(4);
    create_partitions_ahead(&pool, day(2030, 1, 14), 1)
        .await
        .unwrap();

    let mut writer = pool.begin().await.unwrap();
    record(&mut writer, &bus, day(2030, 1, 15), device_event("open"))
        .await
        .unwrap();
    assert_eq!(count_with_lock_timeout(&pool).await.unwrap(), 0);
    writer.rollback().await.unwrap();

    // Control: on a day without a partition the open writer does block the reader.
    let mut writer = pool.begin().await.unwrap();
    record(&mut writer, &bus, day(2030, 1, 20), device_event("open"))
        .await
        .unwrap();
    let blocked = count_with_lock_timeout(&pool).await.unwrap_err();
    let code = blocked
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    assert_eq!(code.as_deref(), Some("55P03"), "{blocked}");
    writer.rollback().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_ensure_partition_succeeds_after_a_racing_creator_commits(pool: PgPool) {
    let at = day(2029, 2, 28);
    let mut first = pool.begin().await.unwrap();
    sqlx::query(ENSURE)
        .bind(at)
        .execute(&mut *first)
        .await
        .unwrap();

    let mut second = pool.acquire().await.unwrap();
    let second_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *second)
        .await
        .unwrap();
    let racing = tokio::spawn(async move {
        sqlx::query(ENSURE)
            .bind(at)
            .execute(&mut *second)
            .await
            .map(|_| ())
    });

    // Wait until the second call is blocked behind the first one's uncommitted partition.
    let mut observer = pool.acquire().await.unwrap();
    let mut waiting = false;
    for _ in 0..500 {
        waiting =
            sqlx::query_scalar("select cardinality(pg_blocking_pids($1)) > 0 from pg_sleep(0.01)")
                .bind(second_pid)
                .fetch_one(&mut *observer)
                .await
                .unwrap();
        if waiting {
            break;
        }
    }
    assert!(waiting, "the second call never waited for the first");
    first.commit().await.unwrap();

    racing.await.unwrap().unwrap();
    assert!(table_exists(&mut observer, "events.event_log_20290228").await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_concurrent_records_on_a_new_day_all_succeed(pool: PgPool) {
    let bus = Bus::new(16);
    let at = day(2029, 2, 28);
    let mut writers = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let (pool, bus) = (pool.clone(), bus.clone());
        writers.spawn(async move {
            let mut tx = pool.begin().await.unwrap();
            let id = record(&mut tx, &bus, at, device_event("racing"))
                .await
                .unwrap();
            tx.commit().await.unwrap();
            id
        });
    }

    let ids = writers.join_all().await;

    let mut conn = pool.acquire().await.unwrap();
    for id in ids {
        assert_eq!(
            partition_of(&mut conn, id).await,
            "events.event_log_20290228"
        );
    }
}

/// Ends the transaction at once (rollback on error), so the other side is not left waiting.
async fn finish<T>(tx: Transaction<'_, Postgres>, outcome: Result<T, Error>) -> Option<String> {
    match outcome {
        Ok(_) => {
            tx.commit().await.unwrap();
            None
        }
        Err(Error::Database(error)) => {
            tx.rollback().await.unwrap();
            let code = error
                .as_database_error()
                .and_then(sqlx::error::DatabaseError::code);
            Some(code.map_or_else(|| error.to_string(), Into::into))
        }
    }
}

/// One transaction writes event then audit, the other audit then event, on the same day; the
/// second statements run concurrently. Returns each side's SQLSTATE, or `None` on success.
async fn write_in_opposite_orders(pool: &PgPool) -> [Option<String>; 2] {
    let (bus, at) = (Bus::new(4), day(2030, 3, 1));
    let mut first = pool.begin().await.unwrap();
    let mut second = pool.begin().await.unwrap();
    record(&mut first, &bus, at, device_event("first"))
        .await
        .unwrap();
    audit(&mut second, at, new_audit(json!({}))).await.unwrap();

    let first_side = async {
        let outcome = audit(&mut first, at, new_audit(json!({}))).await;
        finish(first, outcome).await
    };
    let second_side = async {
        let outcome = record(&mut second, &bus, at, device_event("second")).await;
        finish(second, outcome).await
    };
    let (first, second) = tokio::join!(first_side, second_side);
    [first, second]
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_writes_in_opposite_orders_deadlock_on_a_new_day(pool: PgPool) {
    let mut codes = write_in_opposite_orders(&pool).await;
    codes.sort();
    assert_eq!(codes, [None, Some("40P01".to_owned())]);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_writes_in_opposite_orders_succeed_on_a_day_created_ahead(pool: PgPool) {
    create_partitions_ahead(&pool, day(2030, 3, 1), 0)
        .await
        .unwrap();
    assert_eq!(write_in_opposite_orders(&pool).await, [None, None]);
}
