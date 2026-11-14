//! PRJ-018, LOG-007: after a connect, syncs and their failures, neither the App key nor an
//! installation token is in any table, any process log line, any audit entry or any GraphQL
//! answer.

use anima_hub::fleet::events::process_log;
use serde_json::json;
use sqlx::PgPool;

use super::{CONNECT, TEST_KEY, input, kit, set_projects, stub::Reply};
use crate::logs::Capture;

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

    let pem_line = TEST_KEY.lines().nth(3).unwrap().to_owned();
    let token = kit.stub.token();
    let rows = every_row(&pool).await;
    let log = capture.text();
    assert!(log.contains("source sync failed"), "the log was captured");
    assert!(rows.contains("test.items"), "the rows were read");
    for (place, text) in [("a table", &rows), ("the log", &log)]
        .into_iter()
        .chain(answers.iter().map(|a| ("an answer", a)))
    {
        for (what, needle) in [
            ("the key", pem_line.as_str()),
            ("a key", "PRIVATE KEY"),
            ("the token", token.as_str()),
        ] {
            assert!(!text.contains(needle), "{what} is in {place}");
        }
    }
}
