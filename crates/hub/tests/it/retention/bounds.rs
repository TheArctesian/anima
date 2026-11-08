//! `setRetention` keeps each subject inside its range (D-25), and the Rust range is the one
//! the database constraint holds (DB-011).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{SET, error_json, retention, schema_for, validation_fields, with_variables};

async fn stored_days(pool: &PgPool, subject: &str) -> i32 {
    sqlx::query_scalar("SELECT days FROM retention.periods WHERE subject = $1")
        .bind(subject)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_set_retention_accepts_each_end_and_refuses_one_past_it_on_every_subject(
    pool: PgPool,
) {
    let schema = schema_for(&pool);
    for row in retention(&schema).await {
        let subject = row["subject"].as_str().unwrap();
        let (min, max) = (
            row["minDays"].as_i64().unwrap(),
            row["maxDays"].as_i64().unwrap(),
        );
        for days in [min - 1, max + 1] {
            let before = stored_days(&pool, &subject.to_lowercase()).await;
            let response =
                with_variables(&schema, SET, json!({ "subject": subject, "days": days })).await;
            let error = error_json(&response);
            assert_eq!(validation_fields(&error), ["days"], "{subject} {days}");
            assert_eq!(stored_days(&pool, &subject.to_lowercase()).await, before);
        }
        for days in [min, max] {
            let response =
                with_variables(&schema, SET, json!({ "subject": subject, "days": days })).await;
            assert!(
                response.errors.is_empty(),
                "{subject} {days}: {:?}",
                response.errors
            );
            let data = response.data.into_json().unwrap();
            assert_eq!(data["setRetention"]["days"], days);
            assert_eq!(
                stored_days(&pool, &subject.to_lowercase()).await,
                i32::try_from(days).unwrap()
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_set_retention_refuses_a_value_that_does_not_fit_an_int_range_with_the_same_error(
    pool: PgPool,
) {
    let schema = schema_for(&pool);
    let response = with_variables(&schema, SET, json!({ "subject": "EVENTS", "days": -1 })).await;
    assert_eq!(validation_fields(&error_json(&response)), ["days"]);
    assert_eq!(stored_days(&pool, "events").await, 90);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_011_the_database_constraint_holds_the_range_the_query_reports(pool: PgPool) {
    let schema = schema_for(&pool);
    for row in retention(&schema).await {
        let subject = row["subject"].as_str().unwrap().to_lowercase();
        let (min, max) = (
            row["minDays"].as_i64().unwrap(),
            row["maxDays"].as_i64().unwrap(),
        );
        for (days, accepted) in [(min - 1, false), (min, true), (max, true), (max + 1, false)] {
            let written = sqlx::query("UPDATE retention.periods SET days = $1 WHERE subject = $2")
                .bind(i32::try_from(days).unwrap())
                .bind(&subject)
                .execute(&pool)
                .await;
            let code = written
                .as_ref()
                .err()
                .and_then(|e| e.as_database_error())
                .and_then(sqlx::error::DatabaseError::code);
            assert_eq!(written.is_ok(), accepted, "{subject} {days}");
            if !accepted {
                assert_eq!(code.as_deref(), Some("23514"), "{subject} {days}");
            }
        }
    }
}
