//! The GitHub source for Projects (SRC-006, PRO-003, PRJ-016).
//!
//! Pure: no network, no clock, no hub imports (REPO-E-002). The engine owns the token and
//! the transport. It calls [`begin`] to start a run, asks [`next_request`] what to fetch,
//! fetches it, and hands the response to [`normalise`] with the same slice of state. A run
//! lists the repositories the installation can see, then reads the signs of each one whose
//! `pushed_at` changed: its recursive file tree and its deployment records.

mod app_token;
mod inspection;
mod name;
mod problem;
mod record;
mod repositories;
mod signs;
mod state;

use serde_json::Value;
use source_exchange::{Acquisition, AppKey, Declaration, Minting, Normalised, Request, Response};

use crate::state::{Phase, Slice};

pub use record::{Origin, Record, Repository, Sign, SignKind};

pub const DECLARATION: Declaration = Declaration {
    key: "github",
    name: "GitHub",
    acquisition: Acquisition::TokenPull(Minting::AppKey(AppKey {
        mint_path: app_token::mint_path,
        parse_minted: app_token::parse_minted,
    })),
    scopes: &[
        "Metadata: read",
        "Contents: read (the file list of each repository)",
        "Deployments: read",
    ],
    write_reason: Some(
        "PRJ-017: the operator's own GitHub App has read and write permission; this version only reads.",
    ),
    gives: &[
        "Repositories",
        "Signs of where a repository runs",
        "Deployment records",
    ],
    poll_interval_seconds: 900,
    api_base: "https://api.github.com",
};

/// Starts a run. A finished or empty slice starts a new listing and keeps what was seen; a
/// run that stopped part-way keeps its place, so the next run resumes (SRC-007). A queued
/// repository whose stored name or branch cannot go into a path is dropped (SRC-008).
#[must_use]
pub fn begin(state: &Value) -> Value {
    let mut slice = Slice::read(state);
    slice.pending.retain(|pending| {
        name::check(&pending.name).is_ok() && name::check_branch(&pending.branch).is_ok()
    });
    slice.settle();
    if slice.phase == Phase::Done {
        slice.phase = Phase::List;
        slice.page = 1;
        slice.listed.clear();
    }
    slice.to_value()
}

/// The next request of the run, or `None` when the run is complete.
#[must_use]
pub fn next_request(state: &Value) -> Option<Request> {
    let slice = Slice::read(state);
    match slice.phase {
        Phase::List => repositories::request(slice.page),
        Phase::Signs => slice.pending.first().and_then(inspection::request),
        Phase::Done => None,
    }
}

/// Turns the response to `request` into records, the new slice and problems. A response that
/// is not the answer to [`next_request`] of this slice, or a status with no rule, gives no
/// records, the slice unchanged and a problem: the engine fails the run.
#[must_use]
pub fn normalise(request: &Request, response: &Response<'_>, state: &Value) -> Normalised<Record> {
    let slice = Slice::read(state);
    if next_request(state).as_ref() != Some(request) {
        return out_of_step(&slice);
    }
    match slice.phase {
        Phase::List => repositories::normalise(response, &slice),
        Phase::Signs => inspection::normalise(response, &slice),
        Phase::Done => out_of_step(&slice),
    }
}

fn out_of_step(slice: &Slice) -> Normalised<Record> {
    problem::refuse(
        slice,
        "response_out_of_step",
        "github",
        "the response is not the answer to the request the run was waiting for",
    )
}

/// Takes a repository out of the slice: its queued signs and what was seen of it. The intake
/// calls this for a repository the operator left out, so no tree request is made for it
/// (PRO-002) and restoring it reads its signs again.
#[must_use]
pub fn forget_repository(state: &Value, external_id: &str) -> Value {
    let mut slice = Slice::read(state);
    slice.pending.retain(|pending| pending.id != external_id);
    slice.seen.remove(external_id);
    slice.settle();
    slice.to_value()
}

#[cfg(test)]
mod tests;
