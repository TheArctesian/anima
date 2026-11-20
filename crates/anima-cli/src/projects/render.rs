//! How projects, repositories and resources read in a table. The documents select different
//! parts of the K1 `Project`, so each is rendered from its JSON.

use serde::Serialize;
use serde_json::Value;

use crate::output::{Cell, Table, Tone};

/// The text under `key`, or `-` when it is null or missing.
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("-")
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn many<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A repository as one line: full name, then its marks.
fn repository_line(repository: &Value) -> String {
    let marks: Vec<&str> = [
        ("leftOut", "left out"),
        ("gone", "gone"),
        ("archived", "archived"),
        ("fork", "fork"),
        ("private", "private"),
    ]
    .iter()
    .filter(|(key, _)| flag(repository, key))
    .map(|(_, mark)| *mark)
    .collect();
    if marks.is_empty() {
        text(repository, "fullName").to_owned()
    } else {
        format!("{} ({})", text(repository, "fullName"), marks.join(", "))
    }
}

/// A K1 `ResourceState` in its colour.
fn state(name: &str) -> Cell {
    let tone = if name == "CONFIRMED" {
        Tone::Good
    } else {
        Tone::Warning
    };
    Cell::toned(name, tone)
}

/// A resource as one line: kind, evidence paths and when it was found.
fn resource_line(resource: &Value) -> String {
    let evidence: Vec<String> = many(resource, "evidence")
        .iter()
        .map(|sign| match sign.get("path").and_then(Value::as_str) {
            Some(path) => format!("{} {path}", text(sign, "sign")),
            None => text(sign, "sign").to_owned(),
        })
        .collect();
    format!(
        "{} found {} ({})",
        text(resource, "kind"),
        text(resource, "foundAt"),
        evidence.join(", ")
    )
}

/// The projects of a page as rows: slug, name, repositories, resources, last read.
#[must_use]
pub fn list<T: Serialize>(projects: &T) -> Table {
    let projects = serde_json::to_value(projects).unwrap_or_default();
    let mut table = Table::new(vec!["SLUG", "NAME", "REPOSITORIES", "RESOURCES", "SYNCED"]);
    for edge in many(&projects, "edges") {
        let project = &edge["node"];
        table.row(vec![
            text(project, "slug").into(),
            text(project, "name").into(),
            many(project, "repositories").len().to_string().into(),
            many(project, "resources").len().to_string().into(),
            text(project, "syncedAt").into(),
        ]);
    }
    table
}

/// One project as label and value lines, with its repositories and resources.
#[must_use]
pub fn record<T: Serialize>(project: &T) -> Table {
    let project = serde_json::to_value(project).unwrap_or_default();
    let mut table = Table::record();
    table.pair("Slug", text(&project, "slug"));
    table.pair("Name", text(&project, "name"));
    table.pair("ID", text(&project, "id"));
    table.pair("Synced", text(&project, "syncedAt"));
    for repository in many(&project, "repositories") {
        table.pair("Repository", repository_line(repository));
    }
    for resource in many(&project, "resources") {
        table.pair("Resource", resource_line(resource));
        table.pair("State", state(text(resource, "state")));
    }
    table
}

/// Confirmed resources as rows: ID, kind, state, when confirmed.
#[must_use]
pub fn resources<T: Serialize>(resources: &T) -> Table {
    let resources = serde_json::to_value(resources).unwrap_or_default();
    let mut table = Table::new(vec!["ID", "KIND", "STATE", "CONFIRMED"]);
    for resource in resources.as_array().map_or(&[][..], Vec::as_slice) {
        table.row(vec![
            text(resource, "id").into(),
            text(resource, "kind").into(),
            state(text(resource, "state")),
            text(resource, "confirmedAt").into(),
        ]);
    }
    table
}
