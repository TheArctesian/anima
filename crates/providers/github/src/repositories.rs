//! The listing phase: every repository the installation can see, 100 to a page.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::Value;
use source_exchange::{Body, Normalised, Problem, Request, Response};

use crate::name;
use crate::problem::{problem, refuse, unexpected_status};
use crate::record::{Record, Repository};
use crate::state::{Pending, Phase, Slice, Step};

const PER_PAGE: usize = 100;
/// The most listing pages one listing reads (SRC-007).
const MAX_PAGES: u32 = 1000;
/// The most repositories one listing gathers.
const MAX_REPOSITORIES: usize = 100_000;
const SUBJECT: &str = "repositories";

pub(crate) fn request(page: u32) -> Option<Request> {
    Request::get(
        "/installation/repositories",
        &[("per_page", "100"), ("page", &page.to_string())],
    )
    .ok()
}

#[derive(Deserialize)]
struct Listing {
    total_count: Option<usize>,
    repositories: Vec<Value>,
}

#[derive(Deserialize)]
struct Entry {
    id: u64,
    full_name: String,
    html_url: String,
    default_branch: Option<String>,
    private: bool,
    archived: bool,
    fork: bool,
    description: Option<String>,
    pushed_at: Option<DateTime<Utc>>,
}

/// `text` without U+0000, which no PostgreSQL text or JSON value can hold.
fn nul_free(text: String) -> String {
    if text.contains('\0') {
        text.replace('\0', "")
    } else {
        text
    }
}

/// `value` with U+0000 taken out of every string and key, and nothing else changed, so one
/// entry cannot make its page fail to store. The parser's depth limit bounds the recursion.
fn without_nul(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(nul_free(text)),
        Value::Array(items) => Value::Array(items.into_iter().map(without_nul).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (nul_free(key), without_nul(value)))
                .collect(),
        ),
        other => other,
    }
}

fn to_repository(original: Value) -> Option<Repository> {
    let original = without_nul(original);
    let entry = Entry::deserialize(&original).ok()?;
    Some(Repository {
        external_id: entry.id.to_string(),
        full_name: entry.full_name,
        html_url: entry.html_url,
        default_branch: entry.default_branch.filter(|branch| !branch.is_empty()),
        private: entry.private,
        archived: entry.archived,
        fork: entry.fork,
        description: entry.description,
        pushed_at: entry.pushed_at,
        original,
    })
}

/// The page's repositories and whether it is the last page. `None`: any entry is unreadable,
/// so none is trusted: a missing id would read as a repository that is gone.
fn read_page(body: &[u8], page: u32) -> Option<(Vec<Repository>, bool)> {
    let listing = serde_json::from_slice::<Listing>(body).ok()?;
    let last = listing.repositories.len() < PER_PAGE
        || listing
            .total_count
            .is_some_and(|total| page as usize * PER_PAGE >= total);
    let repositories = listing
        .repositories
        .into_iter()
        .map(to_repository)
        .collect::<Option<Vec<_>>>()?;
    Some((repositories, last))
}

fn stamp(pushed_at: Option<DateTime<Utc>>) -> Option<String> {
    pushed_at.map(|at| at.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// Reads one listing page: a record for each repository, and the slice with the repositories
/// whose `pushed_at` differs from the one their signs were read at queued for the signs phase.
pub(crate) fn normalise(response: &Response<'_>, slice: &Slice) -> Normalised<Record> {
    if response.status != 200 {
        return unexpected_status(slice, response.status, SUBJECT);
    }
    let Body::Read(body) = response.body else {
        return refuse(
            slice,
            "listing_too_large",
            SUBJECT,
            "GitHub's list of repositories is larger than the hub reads",
        );
    };
    let Some((repositories, last)) = read_page(body, slice.page) else {
        return refuse(
            slice,
            "listing_unreadable",
            SUBJECT,
            "GitHub's list of repositories could not be read",
        );
    };
    let mut next = slice.clone();
    let mut problems = Vec::new();
    let mut records = Vec::with_capacity(repositories.len() + 1);
    for repository in repositories {
        next.listed.push(repository.external_id.clone());
        let pushed_at = stamp(repository.pushed_at);
        if next.seen.get(&repository.external_id) != Some(&pushed_at) {
            match queue(&repository, pushed_at) {
                Ok(Some(pending)) => next.pending.push(pending),
                Ok(None) => {}
                Err(unusable) => problems.push(unusable),
            }
        }
        records.push(Record::Repository(repository));
    }
    if !last && (slice.page >= MAX_PAGES || next.listed.len() > MAX_REPOSITORIES) {
        return too_long(slice, records);
    }
    if last {
        let listed = std::mem::take(&mut next.listed);
        next.seen.retain(|id, _| listed.contains(id));
        records.push(Record::Listed {
            external_ids: listed,
        });
        next.page = 1;
        next.phase = Phase::Signs;
        next.settle();
    } else {
        next.page += 1;
    }
    Normalised {
        records,
        state: next.to_value(),
        problems,
    }
}

/// The listing went past the cap: the page's repositories are reported, the run stops with a
/// problem, and the slice drops what this listing gathered so the next run lists from page 1.
/// No `Listed` record: an unfinished listing must not read as one that left repositories out.
fn too_long(slice: &Slice, records: Vec<Record>) -> Normalised<Record> {
    let mut next = slice.clone();
    next.listed.clear();
    next.pending.clear();
    next.page = 1;
    next.phase = Phase::Done;
    Normalised {
        records,
        state: next.to_value(),
        problems: vec![problem(
            "listing_too_long",
            SUBJECT,
            "GitHub's list of repositories did not end within 1,000 pages or 100,000 repositories",
        )],
    }
}

/// `None`: the repository has no branch yet, so there is nothing to read. A problem: its name
/// or its branch cannot go into a path, so its signs are not read.
fn queue(repository: &Repository, pushed_at: Option<String>) -> Result<Option<Pending>, Problem> {
    name::check(&repository.full_name).map_err(|name::UnusableName| {
        problem(
            "repository_name_unusable",
            &repository.full_name,
            "the repository name cannot be used to read its files",
        )
    })?;
    let Some(branch) = &repository.default_branch else {
        return Ok(None);
    };
    name::check_branch(branch).map_err(|name::UnusableBranch| {
        problem(
            "branch_unusable",
            &repository.full_name,
            "the default branch name cannot be used to read its files",
        )
    })?;
    Ok(Some(Pending {
        id: repository.external_id.clone(),
        name: repository.full_name.clone(),
        branch: branch.clone(),
        pushed_at,
        step: Step::Tree,
    }))
}
