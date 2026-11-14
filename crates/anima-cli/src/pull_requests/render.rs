//! How pull requests read in a table. The documents select different parts of the K1
//! `PullRequest`, so each is rendered from its JSON.

use serde::Serialize;
use serde_json::Value;

use crate::forge::comments;
use crate::forge::render::{flag, many, names, number, text};
use crate::output::{Cell, Table, Tone};

/// A K1 `ChecksState` in its colour; `-` when the checks were not asked for.
fn checks(pull: &Value) -> Cell {
    let Some(checks) = pull.get("checks").filter(|checks| !checks.is_null()) else {
        return "-".into();
    };
    let state = text(checks, "state");
    let tone = match state {
        "PASSED" => Tone::Good,
        "FAILED" => Tone::Bad,
        "PENDING" => Tone::Warning,
        _ => Tone::Muted,
    };
    Cell::toned(state, tone)
}

/// `yes` or `no` for whether GitHub says it can merge; `-` when it was not asked.
fn can_merge(pull: &Value) -> &'static str {
    match pull
        .get("mergeability")
        .filter(|mergeability| !mergeability.is_null())
    {
        None => "-",
        Some(mergeability) if flag(mergeability, "canMerge") => "yes",
        Some(_) => "no",
    }
}

/// The pull requests of a page as rows: number, title, author, branches, draft, can merge,
/// checks.
#[must_use]
pub fn list<T: Serialize>(pulls: &T) -> Table {
    let pulls = serde_json::to_value(pulls).unwrap_or_default();
    let mut table = Table::new(vec![
        "NUMBER",
        "TITLE",
        "AUTHOR",
        "BRANCHES",
        "DRAFT",
        "CAN MERGE",
        "CHECKS",
    ]);
    for edge in many(&pulls, "edges") {
        let pull = &edge["node"];
        table.row(vec![
            number(pull, "number").into(),
            text(pull, "title").into(),
            text(pull, "authorLogin").into(),
            format!(
                "{} -> {}",
                text(pull, "headBranch"),
                text(pull, "baseBranch")
            )
            .into(),
            if flag(pull, "draft") { "draft" } else { "-" }.into(),
            can_merge(pull).into(),
            checks(pull),
        ]);
    }
    if !many(&pulls, "mergeMethods").is_empty() {
        let methods = names(&pulls, "mergeMethods");
        table.row(vec!["".into(), format!("merge methods: {methods}").into()]);
    }
    table
}

/// One pull request as label and value lines, with its comments when they were asked for.
#[must_use]
pub fn record<T: Serialize>(pull: &T) -> Table {
    let pull = serde_json::to_value(pull).unwrap_or_default();
    let mut table = Table::record();
    table.pair("Number", number(&pull, "number"));
    table.pair("Title", text(&pull, "title"));
    table.pair("State", text(&pull, "state"));
    table.pair("Author", text(&pull, "authorLogin"));
    table.pair(
        "Branches",
        format!(
            "{} -> {}",
            text(&pull, "headBranch"),
            text(&pull, "baseBranch")
        ),
    );
    table.pair("Head", text(&pull, "headSha"));
    if flag(&pull, "fromFork") {
        table.pair("From a fork", text(&pull, "headRepository"));
    }
    table.pair("Draft", if flag(&pull, "draft") { "yes" } else { "no" });
    table.pair("Can merge", can_merge(&pull));
    if let Some(mergeability) = pull.get("mergeability").filter(|value| !value.is_null()) {
        table.pair("Merge state", text(mergeability, "state"));
    }
    table.pair("Checks", checks(&pull));
    if let Some(checks) = pull.get("checks").filter(|value| !value.is_null()) {
        table.pair(
            "Check counts",
            format!(
                "{} passed, {} failed, {} pending, {} other",
                number(checks, "passed"),
                number(checks, "failed"),
                number(checks, "pending"),
                number(checks, "other")
            ),
        );
        if !many(checks, "failing").is_empty() {
            table.pair("Failing", names(checks, "failing"));
        }
    }
    table.pair("Labels", names(&pull, "labels"));
    table.pair("Comments", number(&pull, "comments"));
    table.pair("Updated", text(&pull, "updatedAt"));
    table.pair("Link", text(&pull, "url"));
    let cut = if flag(&pull, "bodyCut") { " (cut)" } else { "" };
    table.pair("Body", format!("{}{cut}", text(&pull, "body")));
    comments::lines(&mut table, &pull);
    table
}
