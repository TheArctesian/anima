//! The signs phase: for each queued repository, its file tree and then its deployments.

use serde::Deserialize;
use source_exchange::{Normalised, Request, Response};

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
            let Ok(reply) = serde_json::from_slice::<Tree>(response.body) else {
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
            let Ok(reply) = serde_json::from_slice::<Vec<Deployment>>(response.body) else {
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
    let mut next = slice.clone();
    next.pending.remove(0);
    next.seen
        .insert(pending.id.clone(), pending.pushed_at.clone());
    next.settle();
    Normalised {
        records: vec![signs_record(pending, Origin::Deployments, found)],
        state: next.to_value(),
        problems,
    }
}
