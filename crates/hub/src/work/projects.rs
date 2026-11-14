//! The operator's projects (XP-02 first slice; PRO-001 to PRO-004, PRO-006, PRJ-001, PRJ-016).
//!
//! Each repository the GitHub connection can read becomes a project (PRO-001). The engine
//! feeds this domain's [`intake`] while Projects is on; the intake imports repositories, keeps
//! the leave-out list (PRO-002) and turns the signs of a repository into proposed resources
//! (PRO-003, PRO-004). Text from GitHub is stored as given and never interpreted or logged.
//! No domain imports this one; wiring registers the intake and merges [`graphql`].

pub mod graphql;
mod import;
mod intake;
mod model;
mod page;
mod proposals;
mod slug;
mod store;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_graphql::Error as GraphQlError;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::{
    api_errors::{ErrorCode, coded, internal},
    engine::sources::{self, Intake},
    ids::ProjectId,
};

/// The intake the sources engine feeds with what GitHub lists (K-E1 Section 3).
#[must_use]
pub fn intake() -> Arc<dyn Intake> {
    Arc::new(intake::RepositoryIntake)
}

/// Why a projects operation failed. Fixed words: never a repository name or a record.
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("no repository has this ID")]
    NoRepository,
    #[error("no resource has this ID")]
    NoResource,
    #[error("the resource is already confirmed")]
    AlreadyConfirmed,
    #[error("the next sync could not be requested")]
    Sync(#[source] sources::Error),
}

impl Error {
    /// `NOT_FOUND` or `CONFLICT` for what the operator can act on; `INTERNAL` with no detail
    /// otherwise (K7, API-010).
    #[must_use]
    fn into_graphql(self) -> GraphQlError {
        match self {
            Self::NoRepository => coded("No repository has this ID.", ErrorCode::NotFound),
            Self::NoResource => coded("No resource has this ID.", ErrorCode::NotFound),
            Self::AlreadyConfirmed => {
                coded("This resource is already confirmed.", ErrorCode::Conflict)
            }
            other => internal(other),
        }
    }
}

/// Makes the resources of `project` match the signs of its repositories that are not left
/// out: adds found proposals, removes found ones whose last sign went, never touches a
/// confirmed one (PRO-003, PRO-004). Returns whether anything changed.
async fn restate(
    conn: &mut PgConnection,
    project: ProjectId,
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let held = store::signs::held(conn, project).await?;
    let signs = store::signs::of_project(conn, project).await?;
    let plan = proposals::reconcile(&signs, &held);
    store::signs::propose(conn, project, &plan.add, now).await?;
    store::signs::withdraw(conn, &plan.drop).await?;
    Ok(!plan.add.is_empty() || !plan.drop.is_empty())
}
