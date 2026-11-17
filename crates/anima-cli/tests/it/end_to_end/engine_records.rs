//! The timeline and the record links from the command line against the real hub: the event
//! log shows in `anima timeline list` (TML-007), and a link to a record that does not exist
//! keeps its label (RLK-002).

use serde_json::Value;
use sqlx::PgPool;

use super::words;
use crate::kit::Anima;
use crate::kit::real_hub::RealHub;

/// Runs `line` with `--json`; its JSON output.
async fn json_of(anima: &Anima, line: &[&str]) -> Value {
    let mut line = line.to_vec();
    line.push("--json");
    let outcome = anima.run(&line).await;
    assert_eq!(outcome.code, 0, "{line:?}: {outcome:?}");
    serde_json::from_str(&outcome.stdout).unwrap_or_else(|_| panic!("not JSON: {outcome:?}"))
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn tml_007_an_event_of_the_hub_shows_in_anima_timeline_list(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let token = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &token);
    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");

    let events = json_of(&anima, &["event", "list"]).await;
    let timeline = json_of(&anima, &["timeline", "list"]).await;
    let event_ids: Vec<String> = (events["edges"].as_array().expect("events").iter())
        .map(|edge| format!("event:{}", edge["node"]["id"].as_str().expect("an id")))
        .collect();
    let references: Vec<&str> = (timeline["edges"].as_array().expect("entries").iter())
        .map(|edge| edge["node"]["reference"].as_str().expect("a reference"))
        .collect();
    assert_ne!(event_ids.len(), 0, "the site made an event");
    assert_eq!(references, event_ids, "every event, in the same order");
    let entry = &timeline["edges"][0]["node"];
    assert_eq!(entry["source"], "event_log");
    assert_eq!(entry["basis"], "OBSERVED");
    assert_eq!(entry["uncertainty"][0], "End time not given by the source.");

    let table = anima.run(&words("timeline list --category device")).await;
    assert_eq!(table.code, 0, "{table:?}");
    assert!(table.stdout.contains("event_log"), "{}", table.stdout);
    let none = json_of(&anima, &["timeline", "list", "--category", "sleep"]).await;
    assert_eq!(none["edges"].as_array().map(Vec::len), Some(0));
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn rlk_002_a_link_made_from_the_command_line_keeps_its_label_until_it_is_removed(
    pool: PgPool,
) {
    let hub = RealHub::start(pool).await;
    let token = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &token);
    let person = "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";
    let add = [
        "record-link",
        "add",
        "book:1",
        person,
        "--kind",
        "lent_to",
        "--label",
        "Ada Lovelace",
    ];
    let made = json_of(&anima, &add).await;
    assert_eq!(made["label"], "Ada Lovelace", "{made}");
    assert_eq!(made["targetTitle"], Value::Null);

    let listed = json_of(&anima, &["record-link", "list", person]).await;
    assert_eq!(listed[0]["id"], made["id"]);
    assert_eq!(listed[0]["label"], "Ada Lovelace");
    let shown = anima.run(&["record-link", "list", "book:1"]).await;
    assert!(shown.stdout.contains("Ada Lovelace"), "{}", shown.stdout);

    let id = made["id"].as_str().expect("an id");
    let removed = anima.run(&["record-link", "remove", id]).await;
    assert_eq!(removed.code, 0, "{removed:?}");
    let listed = json_of(&anima, &["record-link", "list", person]).await;
    assert_eq!(listed, serde_json::json!([]));
    let again = anima.run(&["record-link", "remove", id]).await;
    assert_ne!(again.code, 0, "{again:?}");
}
