//! `agentSessions` lists the single sessions the daemons sent, the newest use first, with their
//! sums over the days asked for and whether their use is recent (AIS-025, AIS-026).

use chrono::{Days, SecondsFormat, TimeDelta};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Pusher, entry, in_repository, tokens};
use crate::daemon::harness::{DOCKER_HOST, Daemons, PVE_01, pushing_now};

const SESSIONS: &str = "query($filter: AgentUsageFilter, $first: Int!) {
    agentSessions(filter: $filter, first: $first) {
        firstDay lastDay sessionsLeftOut
        sessions { device { id } source model repository startedAt lastUseAt recent
            prompts toolCalls inputTokens outputTokens cacheReadTokens cacheWriteTokens
            estimatedCostUsd unpricedTokens } } }";

async fn sessions(daemons: &Daemons, filter: Value, first: i64) -> Value {
    daemons
        .operate(SESSIONS, json!({ "filter": filter, "first": first }))
        .await
}

/// pve-01: Claude Code session "a" on yesterday and today, last used 10 minutes ago; docker-host:
/// Codex session "b" yesterday, ended at 09:00.
async fn seeded(pool: &PgPool) -> Daemons {
    let now = pushing_now(pool).await;
    let daemons = Daemons::at(pool, now).await;
    let today = now.date_naive();
    let yesterday = today - Days::new(1);
    let stamp = |at: chrono::DateTime<chrono::Utc>| at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let span = |mut one: Value| {
        one["startedAt"] = json!(format!("{yesterday}T08:00:00Z"));
        one["endedAt"] = json!(stamp(now - TimeDelta::minutes(10)));
        in_repository(one, "github.com/example/anima")
    };
    let opus = |input, output| json!([tokens("claude-opus-4-5", [input, output, 0, 0])]);
    Pusher::new(&daemons, PVE_01, now)
        .await
        .reading(json!([
            span(entry(
                ("claude_code", "a"),
                today,
                (4, 10),
                opus(1_000_000, 0)
            )),
            span(entry(
                ("claude_code", "a"),
                yesterday,
                (2, 5),
                opus(0, 1_000_000)
            )),
        ]))
        .await;
    Pusher::new(&daemons, DOCKER_HOST, now)
        .await
        .reading(json!([entry(
            ("codex", "b"),
            yesterday,
            (1, 1),
            json!([tokens("other-model", [10, 0, 0, 0])])
        )]))
        .await;
    daemons
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_025_sessions_come_newest_use_first_with_their_sums_and_estimate(pool: PgPool) {
    let daemons = seeded(&pool).await;
    let answer = sessions(&daemons, json!(null), 50).await;
    let list = &answer["data"]["agentSessions"];
    let rows = list["sessions"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"));
    assert_eq!(rows.len(), 2, "{answer}");
    let first = &rows[0];
    assert_eq!(first["device"]["id"], PVE_01);
    assert_eq!(first["source"], "CLAUDE_CODE");
    assert_eq!(first["repository"], "github.com/example/anima");
    assert_eq!(first["prompts"], 6.0);
    assert_eq!(first["toolCalls"], 15.0);
    assert_eq!(first["inputTokens"], 1_000_000.0);
    assert_eq!(first["outputTokens"], 1_000_000.0);
    // Opus 4.5: 5 dollars for a million input tokens and 25 for a million output tokens.
    assert_eq!(first["estimatedCostUsd"], 30.0);
    assert_eq!(first["recent"], true, "last used 10 minutes ago (AIS-026)");
    let second = &rows[1];
    assert_eq!(second["device"]["id"], DOCKER_HOST);
    assert_eq!(second["source"], "CODEX");
    assert_eq!(second["estimatedCostUsd"], Value::Null);
    assert_eq!(second["unpricedTokens"], 10.0);
    assert_eq!(second["recent"], false);
    assert_eq!(list["sessionsLeftOut"], 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_025_the_filter_and_the_limit_apply_and_the_rest_is_counted(pool: PgPool) {
    let daemons = seeded(&pool).await;
    let one = sessions(&daemons, json!(null), 1).await;
    assert_eq!(
        one["data"]["agentSessions"]["sessions"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "{one}"
    );
    assert_eq!(one["data"]["agentSessions"]["sessionsLeftOut"], 1);
    let codex = sessions(&daemons, json!({ "source": "CODEX" }), 50).await;
    let rows = codex["data"]["agentSessions"]["sessions"]
        .as_array()
        .unwrap_or_else(|| panic!("{codex}"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], "CODEX");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ais_025_a_limit_out_of_range_is_refused(pool: PgPool) {
    let daemons = seeded(&pool).await;
    for first in [0, 201] {
        let answer = sessions(&daemons, json!(null), first).await;
        assert!(answer["errors"].is_array(), "{first}: {answer}");
    }
}
