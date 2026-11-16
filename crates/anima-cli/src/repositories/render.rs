//! How commits, branches and releases read in a table. The commit documents select different
//! parts of the K1 `Commit`, so each is rendered from its JSON.

use serde::Serialize;

use crate::forge::render::{checks, flag, many, text};
use crate::output::Table;

/// The first seven digits of a commit.
fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// The commits of a page as rows: short sha, title, author, date, checks.
#[must_use]
pub fn commits<T: Serialize>(commits: &T) -> Table {
    let commits = serde_json::to_value(commits).unwrap_or_default();
    let mut table = Table::new(vec!["SHA", "TITLE", "AUTHOR", "COMMITTED", "CHECKS"]);
    for edge in many(&commits, "edges") {
        let commit = &edge["node"];
        let author = match commit
            .get("authorLogin")
            .and_then(serde_json::Value::as_str)
        {
            Some(login) => login,
            None => text(commit, "authorName"),
        };
        table.row(vec![
            short(text(commit, "sha")).into(),
            text(commit, "title").into(),
            author.into(),
            text(commit, "committedAt").into(),
            checks(commit),
        ]);
    }
    table
}

/// The branches of a page as rows: name, short sha, whether it is the default, protected.
#[must_use]
pub fn branches<T: Serialize>(branches: &T) -> Table {
    let branches = serde_json::to_value(branches).unwrap_or_default();
    let mut table = Table::new(vec!["NAME", "SHA", "DEFAULT", "PROTECTED"]);
    for edge in many(&branches, "edges") {
        let branch = &edge["node"];
        table.row(vec![
            text(branch, "name").into(),
            short(text(branch, "sha")).into(),
            if flag(branch, "default") { "yes" } else { "-" }.into(),
            if flag(branch, "protected") {
                "yes"
            } else {
                "-"
            }
            .into(),
        ]);
    }
    table
}

/// The releases of a page as rows: tag, name, kind, author, date.
#[must_use]
pub fn releases<T: Serialize>(releases: &T) -> Table {
    let releases = serde_json::to_value(releases).unwrap_or_default();
    let mut table = Table::new(vec!["TAG", "NAME", "KIND", "AUTHOR", "PUBLISHED"]);
    for edge in many(&releases, "edges") {
        let release = &edge["node"];
        let kind = if flag(release, "draft") {
            "draft"
        } else if flag(release, "prerelease") {
            "pre-release"
        } else {
            "release"
        };
        table.row(vec![
            text(release, "tag").into(),
            text(release, "name").into(),
            kind.into(),
            text(release, "authorLogin").into(),
            text(release, "publishedAt").into(),
        ]);
    }
    table
}
