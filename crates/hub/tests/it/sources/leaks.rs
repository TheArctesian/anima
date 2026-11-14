//! PRJ-018, LOG-007: after a connect, syncs and their failures, neither the App key, the App
//! token (the JWT a mint is signed in with) nor an installation token is in any table, any
//! process log line, any audit entry, any GraphQL answer or any `sourceAccountChanged` payload.

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Actor, Bus, Operator, process_log},
    },
    ids::{OperatorId, SourceAccountId},
    schema::{Dependencies, build_schema},
};
use async_graphql::{
    Request, Response, Variables,
    futures_util::{StreamExt as _, stream::BoxStream},
};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{CONNECT, Kit, TEST_KEY, input, kit, set_projects, stub::Reply};
use crate::{
    hub::{dependencies, start},
    logs::Capture,
};

/// Every row of every table, as text.
async fn every_row(pool: &PgPool) -> String {
    let tables: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_schema::text, table_name::text FROM information_schema.tables \
         WHERE table_type = 'BASE TABLE' AND table_schema NOT IN ('pg_catalog', 'information_schema')",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert!(
        tables
            .iter()
            .any(|(s, t)| s == "credentials" && t == "source_secrets")
    );
    let mut text = String::new();
    for (schema, table) in tables {
        let sql = format!(
            "SELECT coalesce(string_agg(t::text, E'\\n'), '') FROM \"{schema}\".\"{table}\" t"
        );
        let rows: String = sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap();
        text.push_str(&rows);
        text.push('\n');
    }
    text
}

/// Every field of the account, its runs and their problems.
const CHANGED: &str = "subscription($id: ID!) { sourceAccountChanged(id: $id) { id sourceKey \
    label instanceKey credentialStored health pausedBy lastSyncAt lastError failureCount \
    nextSyncAt syncing createdAt runs { id trigger startedAt endedAt outcome inserted changed \
    unchanged removed error problems { code subject message } } } }";

/// `sourceAccountChanged` for `id`, on a schema that shares the kit's sources.
fn changes(kit: &Kit, id: SourceAccountId) -> BoxStream<'static, Response> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let schema = build_schema(Dependencies {
        sources: kit.reach.clone(),
        ..dependencies(&kit.pool, clock, Bus::new(16))
    });
    let operator = Operator {
        id: "0190a000-0000-7000-8000-000000000001"
            .parse::<OperatorId>()
            .unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    let request =
        Request::new(CHANGED).variables(Variables::from_json(json!({ "id": id.to_string() })));
    let request = Identity::for_tests(operator, start() + TimeDelta::days(3650)).attach(request);
    schema.execute_stream(request)
}

/// Every payload sent so far, as text; waits until none comes for a while.
async fn payloads(stream: &mut BoxStream<'static, Response>) -> Vec<String> {
    let mut sent = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(300), stream.next()).await
    {
        assert!(next.errors.is_empty(), "{:?}", next.errors);
        sent.push(next.data.into_json().unwrap().to_string());
    }
    sent
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_018_log_007_the_key_and_token_are_nowhere_after_connect_and_sync(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let kit = kit(&pool).await;
    let mut answers = Vec::new();
    let refused = kit
        .hub
        .post_text(
            Some(&kit.token),
            CONNECT,
            input(&TEST_KEY.replace('A', "B")),
        )
        .await;
    answers.push(refused);
    let (connected, id) = kit.connect().await;
    answers.push(connected.to_string());
    let mut stream = changes(&kit, id);
    let first = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
    assert!(first.is_err(), "subscribed; nothing was published yet");
    set_projects(&pool, true).await;
    kit.stub.set_pages(&[&[("a", "1")], &[("b", "2")]]);
    kit.run(id).await;
    let body = format!("{{\"message\":\"Bad credentials {}\"}}", kit.stub.token());
    kit.stub.reply(
        "/pages/1",
        Reply {
            status: 401,
            headers: Vec::new(),
            body,
        },
    );
    kit.run(id).await;
    let query = "query { sourceAccounts { lastError runs { error problems { message } } } }";
    answers.push(kit.hub.post_text(Some(&kit.token), query, json!({})).await);
    let sent = payloads(&mut stream).await;
    assert!(
        sent.len() >= 4,
        "two runs, each started and ended: {sent:?}"
    );

    let pem_line = TEST_KEY.lines().nth(3).unwrap().to_owned();
    let token = kit.stub.token();
    let app_tokens = kit.stub.with(|s| s.app_tokens.clone());
    assert!(!app_tokens.is_empty(), "the mints were seen");
    let rows = every_row(&pool).await;
    let log = capture.text();
    assert!(log.contains("source sync failed"), "the log was captured");
    assert!(rows.contains("test.items"), "the rows were read");
    assert!(
        rows.contains("source_account"),
        "the audit entries were read"
    );
    let mut needles = vec![
        ("the key", pem_line.as_str()),
        ("a key", "PRIVATE KEY"),
        ("the token", token.as_str()),
    ];
    needles.extend(app_tokens.iter().map(|jwt| ("an App token", jwt.as_str())));
    for (place, text) in [("a table", &rows), ("the log", &log)]
        .into_iter()
        .chain(answers.iter().map(|a| ("an answer", a)))
        .chain(sent.iter().map(|p| ("a sourceAccountChanged payload", p)))
    {
        for (what, needle) in &needles {
            assert!(!text.contains(needle), "{what} is in {place}");
        }
    }
}
