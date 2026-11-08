//! The stored periods: seeded with the defaults, read for the daily run (HUB-005, DB-011, D-25).

use anima_hub::retention::{self, Subject};
use chrono::Days;
use sqlx::PgPool;

use super::harness::{retention as listed, schema_for};

async fn stored(pool: &PgPool) -> Vec<(String, i32)> {
    sqlx::query_as("SELECT subject, days FROM retention.periods ORDER BY subject")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn db_011_the_schema_holds_the_defaults_of_d_25(pool: PgPool) {
    let expected = [
        ("deliveries", 30),
        ("events", 90),
        ("heartbeats", 30),
        ("metrics", 30),
        ("recordings", 30),
    ];
    assert_eq!(
        stored(&pool).await,
        expected.map(|(s, d)| (s.to_owned(), d))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_periods_return_the_defaults_after_a_fresh_migration(pool: PgPool) {
    let periods = retention::periods(&pool).await.unwrap();
    let keep: Vec<Days> = Subject::ALL.into_iter().map(|s| periods.keep(s)).collect();
    assert_eq!(keep, [30, 30, 90, 30, 30].map(Days::new));
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_periods_return_the_stored_values(pool: PgPool) {
    for (subject, days) in [
        ("heartbeats", 7),
        ("metrics", 400),
        ("events", 31),
        ("recordings", 1),
        ("deliveries", 365),
    ] {
        sqlx::query("UPDATE retention.periods SET days = $1 WHERE subject = $2")
            .bind(days)
            .bind(subject)
            .execute(&pool)
            .await
            .unwrap();
    }

    let periods = retention::periods(&pool).await.unwrap();

    let keep: Vec<Days> = Subject::ALL.into_iter().map(|s| periods.keep(s)).collect();
    assert_eq!(keep, [7, 400, 31, 1, 365].map(Days::new));
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_a_missing_row_fails_the_read_and_no_default_is_used(pool: PgPool) {
    sqlx::query("DELETE FROM retention.periods WHERE subject = 'events'")
        .execute(&pool)
        .await
        .unwrap();

    let error = retention::periods(&pool).await.unwrap_err();

    assert!(matches!(error, retention::Error::Unreadable), "{error:?}");
    assert_eq!(error.fault(), "unreadable");
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_the_query_lists_every_subject_in_enum_order_with_its_range(pool: PgPool) {
    let rows = listed(&schema_for(&pool)).await;
    let summary: Vec<_> = rows
        .iter()
        .map(|row| {
            let number = |key: &str| row[key].as_i64().unwrap();
            (
                row["subject"].as_str().unwrap().to_owned(),
                [
                    number("days"),
                    number("minDays"),
                    number("maxDays"),
                    number("defaultDays"),
                ],
            )
        })
        .collect();
    let expected = [
        ("HEARTBEATS", [30, 1, 400, 30]),
        ("METRICS", [30, 1, 400, 30]),
        ("EVENTS", [90, 30, 365, 90]),
        ("RECORDINGS", [30, 1, 365, 30]),
        ("DELIVERIES", [30, 7, 365, 30]),
    ];
    assert_eq!(summary, expected.map(|(s, d)| (s.to_owned(), d)));
}
