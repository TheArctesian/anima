//! `agentUsage` sums what the daemons sent by the dimensions asked for, over a range of UTC
//! days, with the estimate of the cost and each daemon's state (AIS-020, AIS-021, AIS-024).

use chrono::{Days, NaiveDate};
use serde_json::{Value, json};
use sqlx::PgPool;

use anima_hub::clock::Clock as _;

use super::{Pusher, USAGE, counts, entry, in_repository, read_key, tokens, usage};
use crate::daemon::harness::{DOCKER_HOST, Daemons, PVE_01, pushing_now};

const ANIMA: &str = "github.com/example/anima";
const SITE: &str = "github.com/example/site";

/// pve-01: a Claude Code session on yesterday and today in `ANIMA`, with Opus 4.5 and a model
/// with no price, and a Codex session today; docker-host: a Claude Code session 29 days ago in
/// `SITE` and one 30 days ago, and then the state `system_unit`.
async fn seeded(pool: &PgPool) -> (Daemons, NaiveDate) {
    let now = pushing_now(pool).await;
    let daemons = Daemons::at(pool, now).await;
    let today = now.date_naive();
    let ago = |days: u64| today - Days::new(days);
    let span = |mut one: Value| {
        one["startedAt"] = json!(format!("{}T08:00:00Z", ago(1)));
        one["endedAt"] = json!(format!("{today}T09:00:00Z"));
        in_repository(one, ANIMA)
    };
    let models = json!([
        tokens("claude-opus-4-5", [1_000_000, 0, 0, 0]),
        tokens("claude-opus-9-9", [100, 200, 0, 0])
    ]);
    let pve = Pusher::new(&daemons, PVE_01, now).await;
    pve.reading(json!([
        span(entry(("claude_code", "a"), today, (4, 10), models)),
        span(entry(
            ("claude_code", "a"),
            ago(1),
            (2, 5),
            json!([tokens("claude-opus-4-5", [0, 1_000_000, 0, 0])])
        )),
        entry(
            ("codex", "b"),
            today,
            (1, 1),
            json!([tokens("gpt-5-codex", [1_000_000, 0, 0, 0])])
        ),
    ]))
    .await;
    let docker = Pusher::new(&daemons, DOCKER_HOST, now).await;
    let sonnet = json!([tokens("claude-sonnet-4-5-20250929", [0, 0, 1_000_000, 0])]);
    docker
        .reading(json!([
            in_repository(entry(("claude_code", "c"), ago(29), (3, 3), sonnet), SITE),
            entry(("claude_code", "d"), ago(30), (100, 100), json!([])),
        ]))
        .await;
    assert_eq!(
        docker.push("system_unit", json!([])).await,
        axum::http::StatusCode::NO_CONTENT
    );
    (daemons, today)
}

fn rows(answer: &Value) -> &Vec<Value> {
    answer["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
}

fn keys<'a>(answer: &'a Value, names: &[&str]) -> Vec<Vec<&'a Value>> {
    rows(answer)
        .iter()
        .map(|row| {
            names
                .iter()
                .map(|name| match *name {
                    "device" => &row["device"]["name"],
                    other => &row[other],
                })
                .collect()
        })
        .collect()
}

fn assert_cost(row: &Value, expected: f64) {
    let cost = row["estimatedCostUsd"]
        .as_f64()
        .unwrap_or_else(|| panic!("{row}"));
    assert!((cost - expected).abs() < 1e-9, "{cost} is not {expected}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_ais_021_the_last_30_days_sum_with_an_estimate_and_each_daemon_state(pool: PgPool) {
    let (daemons, today) = seeded(&pool).await;
    let answer = usage(&daemons, json!({}), &[]).await;
    assert_eq!(answer["firstDay"], (today - Days::new(29)).to_string());
    assert_eq!(answer["lastDay"], today.to_string());
    assert_eq!(answer["pricesAsOf"], "2026-11-17");
    assert_eq!(answer["rowsLeftOut"], 0);
    let total = &answer["total"];
    assert_eq!(
        counts(total),
        [10.0, 19.0, 2_000_100.0, 1_000_200.0, 1_000_000.0, 0.0, 3.0],
        "the session 30 days ago is outside the default range"
    );
    // Opus 4.5: 1M input ($5) and 1M output ($25); GPT-5 Codex: 1M input ($1.25); Sonnet 4.5:
    // 1M cache read ($0.30). claude-opus-9-9 has no price: its 300 tokens are not estimated.
    assert_cost(total, 31.55);
    assert_eq!(total["unpricedTokens"], 300.0);
    assert_eq!(rows(&answer), &vec![total.clone()], "no dimension: one row");
    assert_eq!(
        answer["devices"],
        json!([
            { "device": { "name": "docker-host" }, "state": "SYSTEM_UNIT" },
            { "device": { "name": "pve-01" }, "state": "READING" }
        ])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_rows_group_by_device_source_day_and_model_in_a_fixed_order(pool: PgPool) {
    let (daemons, today) = seeded(&pool).await;
    let by_device = usage(&daemons, json!({}), &["DEVICE", "SOURCE"]).await;
    assert_eq!(
        keys(&by_device, &["device", "source", "sessions", "prompts"]),
        [
            [
                &json!("docker-host"),
                &json!("CLAUDE_CODE"),
                &json!(1),
                &json!(3.0)
            ],
            [
                &json!("pve-01"),
                &json!("CLAUDE_CODE"),
                &json!(1),
                &json!(6.0)
            ],
            [&json!("pve-01"), &json!("CODEX"), &json!(1), &json!(1.0)],
        ]
    );
    let by_day = usage(&daemons, json!({}), &["DAY"]).await;
    let day = |days: u64| json!((today - Days::new(days)).to_string());
    assert_eq!(
        keys(&by_day, &["day", "sessions"]),
        [
            [&day(0), &json!(2)],
            [&day(1), &json!(1)],
            [&day(29), &json!(1)]
        ],
        "newest day first; a session on two days counts on each"
    );
    let by_model = usage(&daemons, json!({}), &["MODEL"]).await;
    assert_eq!(
        keys(
            &by_model,
            &["model", "sessions", "prompts", "unpricedTokens"]
        ),
        [
            [
                &json!("claude-opus-4-5"),
                &json!(1),
                &json!(6.0),
                &json!(0.0)
            ],
            [
                &json!("claude-opus-9-9"),
                &json!(0),
                &json!(0.0),
                &json!(300.0)
            ],
            [
                &json!("claude-sonnet-4-5-20250929"),
                &json!(1),
                &json!(3.0),
                &json!(0.0)
            ],
            [&json!("gpt-5-codex"), &json!(1), &json!(1.0), &json!(0.0)],
        ],
        "a session counts under its model: the one with the most output tokens"
    );
    assert_eq!(rows(&by_model)[1]["estimatedCostUsd"], Value::Null);
    assert_cost(&rows(&by_model)[0], 30.0);
    let by_repository = usage(&daemons, json!({}), &["REPOSITORY"]).await;
    assert_eq!(
        keys(&by_repository, &["repository", "sessions"]),
        [
            [&json!(ANIMA), &json!(1)],
            [&json!(SITE), &json!(1)],
            [&Value::Null, &json!(1)]
        ],
        "no repository last"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_the_filter_narrows_by_device_source_repository_and_days(pool: PgPool) {
    let (daemons, today) = seeded(&pool).await;
    let sessions = |answer: Value| answer["total"]["sessions"].clone();
    let docker = usage(&daemons, json!({ "deviceId": DOCKER_HOST }), &[]).await;
    assert_eq!(sessions(docker.clone()), 1);
    assert_eq!(docker["devices"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        sessions(usage(&daemons, json!({ "source": "CODEX" }), &[]).await),
        1
    );
    assert_eq!(
        sessions(usage(&daemons, json!({ "repository": SITE }), &[]).await),
        1
    );
    let unknown = json!({ "repository": "github.com/example/unknown" });
    assert_eq!(sessions(usage(&daemons, unknown, &[]).await), 0);
    let wider = json!({ "firstDay": (today - Days::new(30)).to_string() });
    assert_eq!(sessions(usage(&daemons, wider, &[]).await), 4);
    let before_today = json!({ "lastDay": (today - Days::new(1)).to_string() });
    let earlier = usage(&daemons, before_today, &[]).await;
    assert_eq!(counts(&earlier["total"])[..2], [105.0, 108.0], "a, c and d");
    assert_eq!(earlier["firstDay"], (today - Days::new(30)).to_string());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_a_bad_filter_is_validation_with_its_fields(pool: PgPool) {
    let (daemons, today) = seeded(&pool).await;
    let first = today - Days::new(366);
    for (filter, fields) in [
        (json!({ "firstDay": "2026-13-01" }), json!(["firstDay"])),
        (json!({ "lastDay": "2026-1-5" }), json!(["lastDay"])),
        (
            json!({ "firstDay": today.succ_opt().unwrap().to_string() }),
            json!(["firstDay"]),
        ),
        (
            json!({ "firstDay": first.to_string() }),
            json!(["firstDay"]),
        ),
        (
            json!({ "repository": "../../etc/passwd" }),
            json!(["repository"]),
        ),
        (json!({ "deviceId": "pve-01" }), json!(["deviceId"])),
    ] {
        let answer = usage(&daemons, filter.clone(), &[]).await;
        let error = &answer["errors"][0];
        assert_eq!(
            error["extensions"]["code"], "VALIDATION",
            "{filter}: {answer}"
        );
        let named: Vec<&Value> = error["extensions"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| &field["field"])
            .collect();
        assert_eq!(json!(named), fields, "{filter}");
    }
    let year = json!({ "firstDay": (today - Days::new(365)).to_string() });
    assert_eq!(
        usage(&daemons, year, &[]).await["total"]["sessions"],
        4,
        "366 days is allowed"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_020_a_read_key_reads_the_usage_and_no_token_reads_nothing(pool: PgPool) {
    let (daemons, _) = seeded(&pool).await;
    let now = daemons.clock.now();
    let key = read_key(&daemons, now).await;
    let variables = json!({ "by": ["SOURCE"] });
    let read = daemons.graphql(Some(&key), USAGE, variables.clone()).await;
    assert_eq!(read["data"]["agentUsage"]["total"]["sessions"], 3, "{read}");
    let refused = daemons.graphql(None, USAGE, variables).await;
    assert_eq!(refused["data"], Value::Null, "{refused}");
    assert!(refused["errors"].is_array(), "{refused}");
}
