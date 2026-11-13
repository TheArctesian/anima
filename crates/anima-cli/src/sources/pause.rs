//! `anima source pause <id-or-label>` and `anima source resume <id-or-label>`.

use super::account::resolve;
use super::documents::{SourcePause, source_pause};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct PauseArgs {
    /// The account's ID or its exact label.
    account: String,
}

#[derive(Debug, clap::Args)]
pub struct ResumeArgs {
    /// The account's ID or its exact label.
    account: String,
}

impl PauseArgs {
    /// Pauses the account's syncs and prints it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_paused(invocation, &self.account, true).await
    }
}

impl ResumeArgs {
    /// Resumes the account's syncs and prints it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown account; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_paused(invocation, &self.account, false).await
    }
}

async fn set_paused(invocation: &Invocation, account: &str, paused: bool) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let named = resolve(&hub, account).await?;
    let variables = source_pause::Variables {
        id: named.id,
        paused,
    };
    let answer = hub.run::<SourcePause>(variables).await?;
    invocation
        .output
        .data(&answer.set_source_account_paused, render::record)
}
