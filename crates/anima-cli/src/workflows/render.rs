//! How workflows and runs read in a table. The documents select different parts of the K1
//! `WorkflowRun`, so each is rendered from its JSON.

use serde::Serialize;
use serde_json::Value;

use crate::forge::render::{flag, many, names, number, text};
use crate::output::{Cell, Table, Tone};

/// The workflows as rows: ID, name, path, whether it is active.
#[must_use]
pub fn workflows<T: Serialize>(workflows: &T) -> Table {
    let workflows = serde_json::to_value(workflows).unwrap_or_default();
    let mut table = Table::new(vec!["ID", "NAME", "PATH", "ACTIVE"]);
    for workflow in workflows.as_array().map_or(&[][..], Vec::as_slice) {
        table.row(vec![
            text(workflow, "id").into(),
            text(workflow, "name").into(),
            text(workflow, "path").into(),
            if flag(workflow, "active") {
                "yes"
            } else {
                "disabled"
            }
            .into(),
        ]);
    }
    table
}

/// A run's status, or its conclusion once it has one, in its colour.
fn outcome(run: &Value) -> Cell {
    let conclusion = run.get("conclusion").and_then(Value::as_str);
    let word = conclusion.unwrap_or_else(|| text(run, "status"));
    let tone = match conclusion {
        Some("SUCCESS") => Tone::Good,
        Some("FAILURE" | "TIMED_OUT") => Tone::Bad,
        Some(_) => Tone::Muted,
        None => Tone::Warning,
    };
    Cell::toned(word, tone)
}

/// `3 passed, 1 failed, ...` for the jobs of a run; `-` when they were not asked for.
fn jobs(run: &Value) -> String {
    let Some(jobs) = run.get("jobs").filter(|jobs| !jobs.is_null()) else {
        return "-".to_owned();
    };
    let counts = [
        ("succeeded", "passed"),
        ("failed", "failed"),
        ("running", "running"),
        ("queued", "queued"),
        ("other", "other"),
    ];
    let parts: Vec<String> = counts
        .iter()
        .filter(|(key, _)| jobs.get(key).and_then(Value::as_i64).unwrap_or(0) > 0)
        .map(|(key, word)| format!("{} {word}", number(jobs, key)))
        .collect();
    let shown = if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(", ")
    };
    if flag(jobs, "partial") {
        format!("{shown} (first 100)")
    } else {
        shown
    }
}

/// The runs of a page as rows: ID, workflow, outcome, branch, commit, event, jobs.
#[must_use]
pub fn runs<T: Serialize>(runs: &T) -> Table {
    let runs = serde_json::to_value(runs).unwrap_or_default();
    let mut table = Table::new(vec![
        "RUN", "WORKFLOW", "OUTCOME", "BRANCH", "COMMIT", "EVENT", "JOBS",
    ]);
    for edge in many(&runs, "edges") {
        let run = &edge["node"];
        let sha: String = text(run, "headSha").chars().take(8).collect();
        table.row(vec![
            text(run, "id").into(),
            text(run, "name").into(),
            outcome(run),
            text(run, "branch").into(),
            format!("{sha} {}", text(run, "commitTitle")).into(),
            text(run, "event").into(),
            jobs(run).into(),
        ]);
    }
    table
}

/// One run as label and value lines, with its jobs.
#[must_use]
pub fn run<T: Serialize>(run: &T) -> Table {
    let run = serde_json::to_value(run).unwrap_or_default();
    let mut table = Table::record();
    table.pair("Run", text(&run, "id"));
    table.pair("Workflow", text(&run, "name"));
    table.pair("Outcome", outcome(&run));
    table.pair("Attempt", number(&run, "attempt"));
    table.pair("Event", text(&run, "event"));
    table.pair("Branch", text(&run, "branch"));
    table.pair("Commit", text(&run, "headSha"));
    table.pair("Commit title", text(&run, "commitTitle"));
    table.pair("Actor", text(&run, "actorLogin"));
    if flag(&run, "fromFork") {
        table.pair("From a fork", "yes");
    }
    table.pair("Started", text(&run, "startedAt"));
    table.pair("Updated", text(&run, "updatedAt"));
    table.pair("Jobs", jobs(&run));
    if let Some(failing) = run
        .get("jobs")
        .filter(|jobs| !many(jobs, "failing").is_empty())
    {
        table.pair("Failing jobs", names(failing, "failing"));
    }
    table.pair("Link", text(&run, "url"));
    table
}
