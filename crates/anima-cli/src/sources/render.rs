//! How a source account and its runs read in a table. The accounts of several documents have
//! one shape here: their JSON, which is the K1 `SourceAccount` with the fields each selected.

use serde::Serialize;
use serde_json::Value;

use crate::output::{Cell, Table, Tone};

/// The text under `key`, or `-` when it is null or missing.
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("-")
}

fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// A K1 `SourceHealth` in its colour.
fn health(name: &str) -> Cell {
    let tone = match name {
        "OK" => Tone::Good,
        "STALLED" | "NEEDS_RECONNECT" => Tone::Bad,
        "NEVER_SYNCED" | "STARTING" => Tone::Warning,
        _ => Tone::Muted,
    };
    Cell::toned(name, tone)
}

/// One run as a line: trigger, outcome, the four counts and when it started.
fn run_line(run: &Value) -> String {
    format!(
        "{} {} +{} ~{} ={} -{} at {}",
        text(run, "trigger"),
        run.get("outcome")
            .and_then(Value::as_str)
            .unwrap_or("RUNNING"),
        number(run, "inserted"),
        number(run, "changed"),
        number(run, "unchanged"),
        number(run, "removed"),
        text(run, "startedAt"),
    )
}

/// The accounts as rows: label, source, health, last sync, next sync and ID.
#[must_use]
pub fn list<T: Serialize>(accounts: &T) -> Table {
    let accounts = serde_json::to_value(accounts).unwrap_or_default();
    let mut table = Table::new(vec![
        "LABEL",
        "SOURCE",
        "HEALTH",
        "LAST SYNC",
        "NEXT SYNC",
        "ID",
    ]);
    for account in accounts.as_array().into_iter().flatten() {
        table.row(vec![
            text(account, "label").into(),
            text(account, "sourceKey").into(),
            health(text(account, "health")),
            text(account, "lastSyncAt").into(),
            text(account, "nextSyncAt").into(),
            text(account, "id").into(),
        ]);
    }
    table
}

/// One account as label and value lines, with its runs when it has them.
#[must_use]
pub fn record<T: Serialize>(account: &T) -> Table {
    let account = serde_json::to_value(account).unwrap_or_default();
    let mut table = Table::record();
    table.pair("Label", text(&account, "label"));
    table.pair("ID", text(&account, "id"));
    table.pair("Source", text(&account, "sourceKey"));
    table.pair("Instance", text(&account, "instanceKey"));
    table.pair("Health", health(text(&account, "health")));
    table.pair("Paused by", text(&account, "pausedBy"));
    table.pair(
        "Syncing",
        if account["syncing"] == true {
            "yes"
        } else {
            "no"
        },
    );
    table.pair("Last sync", text(&account, "lastSyncAt"));
    table.pair("Next sync", text(&account, "nextSyncAt"));
    table.pair("Failures", number(&account, "failureCount").to_string());
    table.pair("Last error", text(&account, "lastError"));
    for run in account["runs"].as_array().into_iter().flatten() {
        table.pair("Run", run_line(run));
        for problem in run["problems"].as_array().into_iter().flatten() {
            table.pair(
                "Problem",
                format!("{}: {}", text(problem, "subject"), text(problem, "message")),
            );
        }
    }
    table
}

/// One line for a change of an account: label, health and last sync.
#[must_use]
pub fn change_line<T: Serialize>(account: &T) -> String {
    let account = serde_json::to_value(account).unwrap_or_default();
    format!(
        "{}  {}  last sync {}{}",
        text(&account, "label"),
        text(&account, "health"),
        text(&account, "lastSyncAt"),
        if account["syncing"] == true {
            "  syncing"
        } else {
            ""
        }
    )
}
