//! The listing phase: every repository the installation can see, 100 to a page.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::Value;
use source_exchange::{Normalised, Request, Response};

use crate::name;
use crate::problem::{problem, refuse, unexpected_status};
use crate::record::{Record, Repository};
use crate::state::{Pending, Phase, Slice, Step};

const PER_PAGE: usize = 100;
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

fn to_repository(original: Value) -> Option<Repository> {
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
    let Some((repositories, last)) = read_page(response.body, slice.page) else {
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
                Err(name::UnusableName) => problems.push(problem(
                    "repository_name_unusable",
                    &repository.full_name,
                    "the repository name cannot be used to read its files",
                )),
            }
        }
        records.push(Record::Repository(repository));
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

/// `None`: the repository has no branch yet, so there is nothing to read.
fn queue(
    repository: &Repository,
    pushed_at: Option<String>,
) -> Result<Option<Pending>, name::UnusableName> {
    name::check(&repository.full_name)?;
    Ok(repository.default_branch.as_ref().map(|branch| Pending {
        id: repository.external_id.clone(),
        name: repository.full_name.clone(),
        branch: branch.clone(),
        pushed_at,
        step: Step::Tree,
    }))
}
