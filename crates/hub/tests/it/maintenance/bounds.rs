//! A stored rule keeps the form's bounds (MW-004, API-005).

use serde_json::{Value, json};
use sqlx::PgPool;

async fn store(pool: &PgPool, kind: &str, rule: Value) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule) \
         VALUES (gen_random_uuid(), $1, $2, 'UTC', $3)",
    )
    .bind(rule.to_string())
    .bind(kind)
    .bind(&rule)
    .execute(pool)
    .await
    .map(|_| ())
}

/// The constraint that refused the write.
fn refused_by(result: Result<(), sqlx::Error>) -> Option<String> {
    let error = result.unwrap_err();
    error
        .as_database_error()
        .and_then(|e| e.constraint())
        .map(str::to_owned)
}

fn weekly(minutes: Value) -> Value {
    json!({ "kind": "recurring_weekday", "weekdays": ["mon"], "startTime": "03:00:00",
            "durationMinutes": minutes })
}

#[sqlx::test(migrations = "./migrations")]
async fn mw_004_a_stored_duration_over_seven_days_is_refused_by_the_database(pool: PgPool) {
    for minutes in [
        json!(4_294_967_295_u64),
        json!(10_081),
        json!(0),
        json!("60"),
    ] {
        let refused = store(&pool, "recurring_weekday", weekly(minutes)).await;
        assert_eq!(
            refused_by(refused).as_deref(),
            Some("windows_rule_bounds_check")
        );
    }
    store(&pool, "recurring_weekday", weekly(json!(10_080)))
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn mw_004_a_stored_list_or_interval_beyond_the_form_bounds_is_refused(pool: PgPool) {
    let days: Vec<i32> = (1..=33).collect();
    let monthly = json!({ "kind": "recurring_day_of_month", "daysOfMonth": days,
                          "startTime": "03:00:00", "durationMinutes": 60 });
    assert!(refused_by(store(&pool, "recurring_day_of_month", monthly).await).is_some());
    let week = json!({ "kind": "recurring_weekday", "weekdays": vec!["mon"; 8],
                       "startTime": "03:00:00", "durationMinutes": 60 });
    assert!(refused_by(store(&pool, "recurring_weekday", week).await).is_some());
    let interval = json!({ "kind": "recurring_interval", "firstStartAt": "2026-11-01T00:00:00Z",
                           "everyDays": 0, "durationMinutes": 60 });
    assert!(refused_by(store(&pool, "recurring_interval", interval).await).is_some());
}
