//! Reads the queued repository’s signs and default branch history.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Body, Normalised, Request, Response};

use crate::name;
use crate::problem::{problem, refuse, unexpected_status};
use crate::record::{Origin, Record, Sign};
use crate::signs;
use crate::state::{Pending, Slice, Step};

/// The request for the step the repository is at. The name was checked when it was queued.
pub(crate) fn request(pending: &Pending) -> Option<Request> {
    match pending.step {
        Step::Tree => Request::get(
            &format!(
                "/repos/{}/git/trees/{}",
                pending.name,
                name::encode_branch(&pending.branch)
            ),
            &[("recursive", "1")],
        ),
        Step::History => Request::get(
            &format!("/repos/{}/commits", pending.name),
            &[("sha", &pending.branch), ("per_page", "1")],
        ),
        Step::Deployments => Request::get(
            &format!("/repos/{}/deployments", pending.name),
            &[("per_page", "30")],
        ),
    }
    .ok()
}

#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeEntry>,
    truncated: bool,
}

#[derive(Deserialize)]
struct TreeEntry {
    path: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Deployment {
    creator: Option<Creator>,
}

#[derive(Deserialize)]
struct Creator {
    login: String,
}

/// Reads the reply for the first queued repository and moves it to its next step.
pub(crate) fn normalise(response: &Response<'_>, slice: &Slice) -> Normalised<Record> {
    let Some(pending) = slice.pending.first() else {
        return refuse(slice, "nothing_queued", "signs", "no repository is waiting");
    };
    match pending.step {
        Step::Tree => tree(response, slice, pending),
        Step::Deployments => deployments(response, slice, pending),
        Step::History => history(response, slice, pending),
    }
}

/// The slice with `pending`, the first queued repository, read for its `pushed_at`, so the
/// next listing queues it again only after a new push.
fn finished(slice: &Slice, pending: &Pending) -> Slice {
    let mut next = slice.clone();
    next.pending.remove(0);
    next.seen
        .insert(pending.id.clone(), pending.pushed_at.clone());
    next.branches
        .insert(pending.id.clone(), pending.branch.clone());
    next.settle();
    next
}

/// A large sign reply skips the remaining signs. Default branch history still gets a turn.
fn too_large(
    slice: &Slice,
    pending: &Pending,
    code: &'static str,
    what: &str,
) -> Normalised<Record> {
    Normalised {
        records: Vec::new(),
        state: next_step(slice, Step::History),
        problems: vec![problem(
            code,
            &pending.name,
            &format!("the repository's {what} is larger than the hub reads"),
        )],
    }
}

fn signs_record(pending: &Pending, origin: Origin, signs: Vec<Sign>) -> Record {
    Record::Signs {
        external_id: pending.id.clone(),
        origin,
        signs,
    }
}

fn tree(response: &Response<'_>, slice: &Slice, pending: &Pending) -> Normalised<Record> {
    let mut next = slice.clone();
    let mut problems = Vec::new();
    let found = match response.status {
        200 => {
            let Body::Read(body) = response.body else {
                return too_large(slice, pending, "tree_too_large", "file list");
            };
            let Ok(reply) = serde_json::from_slice::<Tree>(body) else {
                return refuse(
                    slice,
                    "tree_unreadable",
                    &pending.name,
                    "GitHub's file list could not be read",
                );
            };
            if reply.truncated {
                problems.push(problem(
                    "tree_truncated",
                    &pending.name,
                    "the repository is too large to list; some files may be missing",
                ));
            }
            signs::from_paths(
                reply
                    .tree
                    .iter()
                    .filter(|entry| entry.kind == "blob")
                    .map(|entry| entry.path.as_str()),
            )
        }
        // An empty repository has no tree to read.
        409 => Vec::new(),
        404 => {
            problems.push(problem(
                "tree_not_found",
                &pending.name,
                "GitHub has no file list for the repository",
            ));
            Vec::new()
        }
        status => return unexpected_status(slice, status, &pending.name),
    };
    next.pending[0].step = Step::Deployments;
    Normalised {
        records: vec![signs_record(pending, Origin::Tree, found)],
        state: next.to_value(),
        problems,
    }
}

fn deployments(response: &Response<'_>, slice: &Slice, pending: &Pending) -> Normalised<Record> {
    let mut problems = Vec::new();
    let found = match response.status {
        200 => {
            let Body::Read(body) = response.body else {
                return too_large(
                    slice,
                    pending,
                    "deployments_too_large",
                    "list of deployments",
                );
            };
            let Ok(reply) = serde_json::from_slice::<Vec<Deployment>>(body) else {
                return refuse(
                    slice,
                    "deployments_unreadable",
                    &pending.name,
                    "GitHub's list of deployments could not be read",
                );
            };
            signs::from_creators(
                reply
                    .iter()
                    .filter_map(|deployment| deployment.creator.as_ref())
                    .map(|creator| creator.login.as_str()),
            )
        }
        // The App may lack the deployments permission; the files are still worth having.
        403 | 404 => {
            problems.push(problem(
                "deployments_unavailable",
                &pending.name,
                "GitHub did not give the list of deployments",
            ));
            Vec::new()
        }
        status => return unexpected_status(slice, status, &pending.name),
    };
    Normalised {
        records: vec![signs_record(pending, Origin::Deployments, found)],
        state: next_step(slice, Step::History),
        problems,
    }
}

fn next_step(slice: &Slice, step: Step) -> serde_json::Value {
    let mut next = slice.clone();
    next.pending[0].step = step;
    next.to_value()
}

#[derive(Deserialize)]
struct CommitEntry {
    commit: Commit,
}

#[derive(Deserialize)]
struct Commit {
    committer: Committer,
}

#[derive(Deserialize)]
struct Committer {
    date: DateTime<Utc>,
}

fn history_facts(response: &Response<'_>) -> Option<(Option<DateTime<Utc>>, Option<i32>)> {
    if response.status == 409 {
        return Some((None, Some(0)));
    }
    let Body::Read(body) = response.body else {
        return None;
    };
    let commits: Vec<CommitEntry> = serde_json::from_slice(body).ok()?;
    let Some(tip) = commits.first() else {
        return Some((None, Some(0)));
    };
    let count = response.link.map_or(Some(1), |link| {
        crate::live::paging::related_page(link, "last").and_then(|page| i32::try_from(page).ok())
    });
    Some((Some(tip.commit.committer.date), count))
}

fn history(response: &Response<'_>, slice: &Slice, pending: &Pending) -> Normalised<Record> {
    let state = finished(slice, pending).to_value();
    // The engine handles rate limits before it calls the provider.
    if matches!(response.status, 403 | 404) {
        return Normalised {
            records: Vec::new(),
            state,
            problems: vec![problem(
                "history_unavailable",
                &pending.name,
                "GitHub did not give the default branch history",
            )],
        };
    }
    if !matches!(response.status, 200 | 409) {
        return unexpected_status(slice, response.status, &pending.name);
    }
    let Some((last_commit_at, commit_count)) = history_facts(response) else {
        return Normalised {
            records: Vec::new(),
            state,
            problems: vec![problem(
                "history_unreadable",
                &pending.name,
                "GitHub's default branch history could not be read",
            )],
        };
    };
    Normalised {
        records: vec![Record::History {
            external_id: pending.id.clone(),
            branch: pending.branch.clone(),
            last_commit_at,
            commit_count,
        }],
        state,
        problems: Vec::new(),
    }
}
