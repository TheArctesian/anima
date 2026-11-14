//! Tests run from JSON fixtures under `tests/fixtures`, shaped like GitHub's replies.

use serde_json::{Value, json};
use source_exchange::{Normalised, Problem, Request, Response};

use super::*;

mod branches;
mod flow;
mod inspection;
mod listing;
mod sign_rules;
mod token;

pub(super) fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

/// The fixture repository with its id, name and `pushed_at` changed.
pub(super) fn repository(id: u64, full_name: &str, pushed_at: &str) -> Value {
    let mut entry: Value = serde_json::from_slice(&fixture("repository.json")).unwrap();
    entry["id"] = json!(id);
    entry["full_name"] = json!(full_name);
    entry["name"] = json!(full_name.rsplit('/').next().unwrap());
    entry["pushed_at"] = json!(pushed_at);
    entry
}

pub(super) fn listing(total: usize, repositories: &[Value]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "total_count": total,
        "repository_selection": "all",
        "repositories": repositories,
    }))
    .unwrap()
}

pub(super) fn empty_tree() -> Vec<u8> {
    br#"{"sha":"0","tree":[],"truncated":false}"#.to_vec()
}

/// What the stand-in GitHub answers: listing pages by number, then one answer for every
/// tree request and one for every deployments request.
pub(super) struct Replies {
    pub pages: Vec<Vec<u8>>,
    pub tree: (u16, Vec<u8>),
    pub deployments: (u16, Vec<u8>),
}

impl Replies {
    pub fn new(pages: Vec<Vec<u8>>) -> Self {
        Self {
            pages,
            tree: (200, empty_tree()),
            deployments: (200, b"[]".to_vec()),
        }
    }

    fn answer(&self, request: &Request) -> (u16, Vec<u8>) {
        if request.path == "/installation/repositories" {
            let page: usize = query(request, "page").parse().unwrap();
            (200, self.pages[page - 1].clone())
        } else if request.path.contains("/git/trees/") {
            self.tree.clone()
        } else {
            self.deployments.clone()
        }
    }
}

pub(super) fn query<'a>(request: &'a Request, name: &str) -> &'a str {
    request
        .query
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value.as_str())
}

pub(super) struct Run {
    pub records: Vec<Record>,
    pub problems: Vec<Problem>,
    pub requests: Vec<Request>,
    pub state: Value,
}

/// One run as the engine drives it: begin, then request and normalise until none is left.
pub(super) fn run(state: &Value, replies: &Replies) -> Run {
    let mut state = begin(state);
    let mut done = Run {
        records: Vec::new(),
        problems: Vec::new(),
        requests: Vec::new(),
        state: Value::Null,
    };
    while let Some(request) = next_request(&state) {
        assert!(done.requests.len() < 1000, "the run does not end");
        let (status, body) = replies.answer(&request);
        let out = normalise(
            &request,
            &Response {
                status,
                body: &body,
            },
            &state,
        );
        done.requests.push(request);
        done.records.extend(out.records);
        let stuck = out.state == state;
        done.problems.extend(out.problems);
        state = out.state;
        if stuck {
            break;
        }
    }
    done.state = state;
    done
}

pub(super) fn signs_of(run: &Run, id: &str, origin: Origin) -> Vec<Sign> {
    run.records
        .iter()
        .find_map(|record| match record {
            Record::Signs {
                external_id,
                origin: found,
                signs,
            } if external_id == id && *found == origin => Some(signs.clone()),
            _ => None,
        })
        .unwrap()
}

pub(super) fn tree_only(kind: SignKind, path: &str) -> Vec<Sign> {
    vec![Sign {
        kind,
        path: Some(path.to_owned()),
    }]
}

pub(super) const ANIMA: &str = "612345678";

/// One listing page holding the fixture repository.
pub(super) fn one_repository() -> Vec<Vec<u8>> {
    vec![listing(
        1,
        &[repository(
            612_345_678,
            "arctesian/anima",
            "2026-11-10T17:01:58Z",
        )],
    )]
}
