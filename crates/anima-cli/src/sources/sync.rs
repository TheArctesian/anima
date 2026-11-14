//! `anima source sync <id-or-label>`: starts a sync now (ITG-002).

use super::account::resolve;
use super::documents::{SourceSync, source_sync};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    /// The account's ID or its exact label.
    account: String,
}

impl SyncArgs {
    /// Starts the sync, which the hub runs on its own, and prints the account.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; the hub's refusal (`CONFLICT` while the
    /// account is paused).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let named = resolve(&hub, &self.account).await?;
        let answer = hub
            .run::<SourceSync>(source_sync::Variables { id: named.id })
            .await?;
        invocation
            .output
            .data(&answer.sync_source_account, render::record)
    }
}
