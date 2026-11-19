//! Dawarich as a source of location points (K-E4 Section 11; K-E1 Sections 2 and 7b; R-145,
//! R-210). Pure: it names the request it wants and reads the reply that the engine hands
//! back. The wire form is built from the documents of Dawarich; no code of it is copied.

mod point;
#[cfg(test)]
mod tests;

use chrono::{DateTime, SecondsFormat};
use serde_json::{Value, json};
use source_exchange::{
    AccountKey, Acquisition, Body, Declaration, Normalised, Problem, Request, Response,
};

pub use point::Point;

/// The points of one page.
pub const PER_PAGE: usize = 1000;
/// The version of the sync state.
const VERSION: u64 = 1;
const POINTS: &str = "/api/v1/points";
/// The end of each read: later than any point.
const END: &str = "2100-01-01T00:00:00Z";

/// The request that proves a key at connect: one point of the account.
fn probe() -> Request {
    Request::get(POINTS, &[("per_page", "1")]).expect("a fixed path below the base")
}

pub const DECLARATION: Declaration = Declaration {
    key: "dawarich",
    name: "Dawarich",
    acquisition: Acquisition::AccountKey(AccountKey { probe }),
    scopes: &["Read the location points of the account"],
    write_reason: None,
    gives: &["Location samples"],
    poll_interval_seconds: 900,
    api_base: "",
    changes: None,
};

/// The time of the last point that a finished run stored, in Unix seconds; 0 before the first.
fn since(state: &Value) -> i64 {
    state["since"].as_i64().unwrap_or(0)
}

/// The state at the start of a run: the time of the last stored point, and page 1. A state of
/// an unknown version starts from the first point.
#[must_use]
pub fn begin(state: &Value) -> Value {
    let known = state["v"].as_u64() == Some(VERSION);
    let since = if known { since(state) } else { 0 };
    json!({ "v": VERSION, "since": since, "newest": since, "page": 1, "done": false })
}

/// The next page of the run; `None` when the last page was read.
#[must_use]
pub fn next_request(state: &Value) -> Option<Request> {
    if state["done"].as_bool().unwrap_or(true) {
        return None;
    }
    // One second before the last point: a point of the same second is read again, and the
    // hub stores a point that it has one time.
    let start = DateTime::from_timestamp((since(state) - 1).max(0), 0)?
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let page = state["page"].as_u64().unwrap_or(1).to_string();
    let per_page = PER_PAGE.to_string();
    let query = [
        ("start_at", start.as_str()),
        ("end_at", END),
        ("order", "asc"),
        ("page", page.as_str()),
        ("per_page", per_page.as_str()),
    ];
    Request::get(POINTS, &query).ok()
}

fn problem(code: &'static str, message: &str) -> Problem {
    Problem {
        code,
        subject: "points".to_owned(),
        message: message.to_owned(),
    }
}

/// The state unchanged with one problem: the engine fails the run, and the next run asks for
/// this page again.
fn stuck(state: &Value, code: &'static str, message: &str) -> Normalised<Point> {
    Normalised {
        records: Vec::new(),
        state: state.clone(),
        problems: vec![problem(code, message)],
    }
}

/// The points of one page and the state after it. A page with fewer than [`PER_PAGE`] points
/// is the last one: the time of the newest point becomes the start of the next run. A point
/// with no `id` is a problem and is not a record. The same input gives the same output.
#[must_use]
pub fn normalise(_request: &Request, response: &Response<'_>, state: &Value) -> Normalised<Point> {
    let Body::Read(body) = response.body else {
        return stuck(
            state,
            "too_large",
            "Dawarich sent a page larger than the hub reads.",
        );
    };
    if response.status != 200 {
        return stuck(state, "status", "Dawarich did not give the points.");
    }
    let Ok(Value::Array(entries)) = serde_json::from_slice::<Value>(body) else {
        return stuck(
            state,
            "form",
            "Dawarich sent a page that is not a list of points.",
        );
    };
    let mut records = Vec::new();
    let mut without_id = 0_usize;
    for entry in &entries {
        match Point::read(entry) {
            Some(point) => records.push(point),
            None => without_id += 1,
        }
    }
    let seen = records.iter().filter_map(|point| point.timestamp).max();
    let newest = state["newest"].as_i64().unwrap_or(0).max(seen.unwrap_or(0));
    let last = entries.len() < PER_PAGE;
    let state = json!({
        "v": VERSION,
        "since": if last { newest } else { since(state) },
        "newest": newest,
        "page": if last { 1 } else { state["page"].as_u64().unwrap_or(1) + 1 },
        "done": last,
    });
    let problems = if without_id == 0 {
        Vec::new()
    } else {
        vec![problem(
            "no_id",
            &format!("{without_id} point(s) had no id and were not stored."),
        )]
    };
    Normalised {
        records,
        state,
        problems,
    }
}
