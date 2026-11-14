//! `anima project leave-out <repository>` and `anima project restore <repository>` (PRJ-002).

use super::documents::{
    RepositoryLeaveOut, RepositoryRestore, repository_leave_out, repository_restore,
};
use super::render;
use super::repository::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct LeaveOutArgs {
    /// The repository's full name (owner/name) or its ID.
    repository: String,
}

#[derive(Debug, clap::Args)]
pub struct RestoreArgs {
    /// The repository's full name (owner/name) or its ID.
    repository: String,
}

impl LeaveOutArgs {
    /// Leaves the repository out and prints its project.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown repository; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.repository).await?;
        let answer = hub
            .run::<RepositoryLeaveOut>(repository_leave_out::Variables { id })
            .await?;
        invocation
            .output
            .data(&answer.leave_out_repository, render::record)
    }
}

impl RestoreArgs {
    /// Restores the repository and prints its project.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown repository; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.repository).await?;
        let answer = hub
            .run::<RepositoryRestore>(repository_restore::Variables { id })
            .await?;
        invocation
            .output
            .data(&answer.restore_repository, render::record)
    }
}
