//! The Projects intake of the GitHub source: it runs the provider over each page and carries
//! out the import rule in the engine's transaction (K-E1 Section 3; PRO-001 to PRO-004).

use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use provider_github::{Origin, Record, Repository};
use serde_json::Value;
use source_exchange::{Problem, Request};
use sqlx::PgConnection;

use super::{
    import::{self, Step},
    model::{Canonical, Sign, origin_text},
    restate, slug,
    store::{self, signs::Found},
};
use crate::{
    engine::{
        extensions::{Key, PROJECTS},
        sources::{Counts, Intake, IntakeError, Page, Taken},
    },
    ids::SourceAccountId,
};

/// Reads the repositories of a GitHub installation for Projects.
pub(super) struct RepositoryIntake;

fn canonical(repository: &Repository) -> Canonical {
    Canonical {
        full_name: repository.full_name.clone(),
        html_url: repository.html_url.clone(),
        default_branch: repository.default_branch.clone(),
        private: repository.private,
        archived: repository.archived,
        fork: repository.fork,
        description: repository.description.clone(),
        pushed_at: repository.pushed_at,
    }
}

/// Carries out the import rule for one listed repository; `None` when its name is unusable.
async fn take_repository(
    conn: &mut PgConnection,
    account: SourceAccountId,
    repository: &Repository,
    now: DateTime<Utc>,
) -> Result<Option<Step>, sqlx::Error> {
    let full_name = repository.full_name.as_str();
    if !import::usable(full_name) {
        return Ok(None);
    }
    let known = store::known(conn, &repository.external_id).await?;
    let step = import::step(known.as_ref(), &canonical(repository));
    let name = import::project_name(full_name);
    match (step, known) {
        (Step::Import, _) => {
            let taken = store::slugs_like(conn, &slug::stem(full_name)).await?;
            let slug = slug::for_repository(full_name, &taken);
            store::import(conn, (&slug, name), (account, repository), now).await?;
        }
        (Step::Refresh, Some(known)) => {
            store::refresh(conn, (known.id, account), (repository, name), now).await?;
        }
        (Step::Return, Some(known)) => store::returned(conn, known.id, now).await?,
        _ => {}
    }
    Ok(Some(step))
}

/// Stores the signs of one origin of a repository and restates its project's proposals.
/// Returns whether they changed. A left-out or unknown repository gets nothing (PRO-002).
async fn take_signs(
    conn: &mut PgConnection,
    (external_id, origin): (&str, Origin),
    signs: Vec<provider_github::Sign>,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let Some(known) = store::known(conn, external_id).await? else {
        return Ok(false);
    };
    if known.left_out {
        return Ok(false);
    }
    let origin = origin_text(origin);
    let fresh: BTreeSet<Found> = signs
        .into_iter()
        .map(|sign| (Sign::of(sign.kind), sign.path))
        .collect();
    let stored: BTreeSet<Found> = store::signs::of_origin(conn, known.id, origin)
        .await?
        .into_iter()
        .collect();
    if fresh == stored {
        return Ok(false);
    }
    let fresh: Vec<Found> = fresh.into_iter().collect();
    store::signs::replace(conn, (known.id, origin), &fresh).await?;
    restate(conn, known.project_id, now).await?;
    Ok(true)
}

fn unusable(repository: &Repository) -> Problem {
    Problem {
        code: "repository_unusable",
        subject: repository.external_id.clone(),
        message: "the repository name is not owner/name of at most 300 characters".to_owned(),
    }
}

/// Stores one record; adds what it did to `counts`.
async fn take_record(
    conn: &mut PgConnection,
    page: &Page<'_>,
    record: Record,
    (counts, problems): (&mut Counts, &mut Vec<Problem>),
) -> Result<(), sqlx::Error> {
    match record {
        Record::Repository(repository) => {
            match take_repository(conn, page.account, &repository, page.now).await? {
                Some(Step::Import) => counts.inserted += 1,
                Some(Step::Refresh | Step::Return) => counts.changed += 1,
                Some(Step::Keep) => counts.unchanged += 1,
                None => problems.push(unusable(&repository)),
            }
        }
        Record::Listed { external_ids } => {
            let gone = store::listed(conn, page.account, &external_ids, page.now).await?;
            counts.removed += u32::try_from(gone).unwrap_or(u32::MAX);
        }
        Record::Signs {
            external_id,
            origin,
            signs,
        } => {
            if take_signs(conn, (&external_id, origin), signs, page.now).await? {
                counts.changed += 1;
            } else {
                counts.unchanged += 1;
            }
        }
    }
    Ok(())
}

#[async_trait]
impl Intake for RepositoryIntake {
    fn source(&self) -> &'static str {
        provider_github::DECLARATION.key
    }

    fn extension(&self) -> Key {
        PROJECTS
    }

    fn slice_key(&self) -> &'static str {
        "github.projects"
    }

    fn begin(&self, slice: &Value) -> Value {
        provider_github::begin(slice)
    }

    fn next_request(&self, slice: &Value) -> Option<Request> {
        provider_github::next_request(slice)
    }

    /// Stores the page's records, then takes every left-out repository out of the new slice,
    /// so the run asks for none of its files (PRO-002) and a restore reads them again.
    async fn take(&self, conn: &mut PgConnection, page: Page<'_>) -> Result<Taken, IntakeError> {
        let normalised = provider_github::normalise(page.request, &page.response, page.slice);
        let mut counts = Counts::default();
        let mut problems = normalised.problems;
        for record in normalised.records {
            take_record(conn, &page, record, (&mut counts, &mut problems)).await?;
        }
        let mut slice = normalised.state;
        for external_id in store::left_out_ids(conn).await? {
            slice = provider_github::forget_repository(&slice, &external_id);
        }
        Ok(Taken {
            slice,
            counts,
            problems,
        })
    }

    async fn forget(
        &self,
        conn: &mut PgConnection,
        account: SourceAccountId,
    ) -> Result<u64, IntakeError> {
        Ok(store::forget(conn, account).await?)
    }
}
