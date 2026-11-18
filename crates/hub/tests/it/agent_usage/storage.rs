//! How the hub stores the `agentUsage` of a batch (K8 Section 4; AIS-020, AIS-023, AIS-024):
//! an idempotent upsert of each session day, the state, and nothing of a device once deleted.

use axum::http::StatusCode;
use chrono::{Days, NaiveDate};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Pusher, counts, entry, in_repository, tokens, usage};
use crate::daemon::harness::{Daemons, PVE_01, pushing_now};

const SESSION: (&str, &str) = ("claude_code", "6f0c2d9e-3b1a-4c55-9e7d-2a8b4c6d8e10");

fn opus_and_haiku(day: NaiveDate) -> Value {
    let models = json!([
        tokens("claude-opus-4-5", [1000, 2000, 30_000, 400]),
        tokens("claude-haiku-4-5", [10, 20, 0, 0])
    ]);
    in_repository(
        entry(SESSION, day, (6, 41), models),
        "github.com/example/anima",
    )
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_a_resent_entry_changes_nothing_and_changed_counts_replace_its_day(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let pusher = Pusher::new(&daemons, PVE_01, now).await;
    let day = now.date_naive() - Days::new(1);

    pusher.reading(json!([opus_and_haiku(day)])).await;
    let first = usage(&daemons, json!({}), &["MODEL"]).await;
    assert_eq!(
        counts(&first["total"]),
        [6.0, 41.0, 1010.0, 2020.0, 30_000.0, 400.0, 1.0]
    );
    pusher.reading(json!([opus_and_haiku(day)])).await;
    assert_eq!(
        usage(&daemons, json!({}), &["MODEL"]).await,
        first,
        "the same entry again changes nothing"
    );

    let changed = json!([tokens("claude-opus-4-5", [1500, 2500, 0, 0])]);
    let resent = in_repository(
        entry(SESSION, day, (8, 50), changed),
        "github.com/example/anima",
    );
    pusher.reading(json!([resent])).await;
    let replaced = usage(&daemons, json!({}), &["MODEL"]).await;
    assert_eq!(
        counts(&replaced["total"]),
        [8.0, 50.0, 1500.0, 2500.0, 0.0, 0.0, 1.0],
        "the entry replaces the day's counts; they are not added"
    );
    let models: Vec<&Value> = replaced["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["model"])
        .collect();
    assert_eq!(models, [&json!("claude-opus-4-5")], "haiku left the day");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_a_session_takes_its_start_end_and_repository_from_the_entry_stored_last(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let pusher = Pusher::new(&daemons, PVE_01, now).await;
    let today = now.date_naive();
    let yesterday = today - Days::new(1);
    let mut earlier = opus_and_haiku(yesterday);
    earlier["endedAt"] = json!(format!("{today}T01:00:00Z"));
    let mut later = entry(SESSION, today, (1, 2), json!([]));
    later["startedAt"] = earlier["startedAt"].clone();
    later["endedAt"] = json!(format!("{today}T01:00:00Z"));
    // Both days in one batch: the last entry of the session sets its repository (none).
    pusher.reading(json!([earlier, later])).await;

    let by_repository = usage(&daemons, json!({}), &["REPOSITORY"]).await;
    let rows = by_repository["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{by_repository}");
    assert_eq!(
        rows[0]["repository"],
        Value::Null,
        "no repository clears it"
    );
    assert_eq!(counts(&rows[0])[..2], [7.0, 43.0], "both days are kept");

    pusher.reading(json!([opus_and_haiku(yesterday)])).await;
    let again = usage(&daemons, json!({}), &["REPOSITORY", "DAY"]).await;
    let repositories: Vec<(&Value, &Value)> = again["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (&row["day"], &row["repository"]))
        .collect();
    let repository = json!("github.com/example/anima");
    assert_eq!(
        repositories,
        [
            (&json!(today.to_string()), &repository),
            (&json!(yesterday.to_string()), &repository)
        ],
        "the session's repository is the one of its entry stored last, for all its days"
    );
    let ended: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT ended_at FROM agent_usage.sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ended.to_string(), format!("{yesterday} 09:00:00 UTC"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_024_an_unknown_source_is_dropped_and_an_unknown_state_keeps_the_stored_one(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let pusher = Pusher::new(&daemons, PVE_01, now).await;
    let day = now.date_naive();
    let codex = entry(("codex", "019a-root"), day, (2, 3), json!([]));
    let newer = entry(("gemini_cli", "abc"), day, (100, 100), json!([]));
    pusher.reading(json!([codex, newer])).await;
    let read = usage(&daemons, json!({}), &["SOURCE"]).await;
    let sources: Vec<&Value> = read["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["source"])
        .collect();
    assert_eq!(
        sources,
        [&json!("CODEX")],
        "a source the hub does not know is dropped"
    );
    let state = |answer: &Value| answer["devices"].clone();
    let reading = json!([{ "device": { "name": "pve-01" }, "state": "READING" }]);
    assert_eq!(state(&read), reading);

    assert_eq!(
        pusher.push("sleeping", json!([])).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        state(&usage(&daemons, json!({}), &[]).await),
        reading,
        "a state the hub does not know is ignored"
    );
    assert_eq!(
        pusher.push("system_unit", json!([])).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        state(&usage(&daemons, json!({}), &[]).await),
        json!([{ "device": { "name": "pve-01" }, "state": "SYSTEM_UNIT" }])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_018_an_entry_that_breaks_k8_is_malformed_and_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let pusher = Pusher::new(&daemons, PVE_01, now).await;
    let day = now.date_naive();
    let hostile_model = json!([tokens(
        "x'); DROP TABLE agent_usage.sessions; --",
        [1, 1, 1, 1]
    )]);
    let with_path = in_repository(
        entry(SESSION, day, (1, 1), json!([])),
        "https://user:secret@github.com/example/anima.git",
    );
    for session_days in [
        json!([entry(SESSION, day, (1, 1), hostile_model)]),
        json!([with_path]),
    ] {
        assert_eq!(
            pusher.push("reading", session_days).await,
            StatusCode::BAD_REQUEST
        );
    }
    for table in ["states", "sessions", "session_days", "day_models"] {
        let stored: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM agent_usage.{table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, 0, "{table}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_deleting_a_device_deletes_its_sessions_and_its_state(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let pusher = Pusher::new(&daemons, PVE_01, now).await;
    pusher
        .reading(json!([opus_and_haiku(now.date_naive())]))
        .await;
    sqlx::query("DELETE FROM devices.devices WHERE id = $1::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();
    for table in ["states", "sessions", "session_days", "day_models"] {
        let left: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM agent_usage.{table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0, "{table}");
    }
    let answer = usage(&daemons, json!({}), &[]).await;
    assert_eq!(answer["devices"], json!([]));
    assert_eq!(answer["total"]["sessions"], 0);
}
