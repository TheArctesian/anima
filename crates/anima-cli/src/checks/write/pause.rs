//! `anima check pause` and `anima check resume`: stop or restart a check's runs.

use super::documents::{CheckSetPaused, check_set_paused};
use crate::checks::resolve::CheckRef;
use crate::checks::summary::{state, written};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct PauseArgs {
    #[command(flatten)]
    check: CheckRef,
}

#[derive(Debug, clap::Args)]
pub struct ResumeArgs {
    #[command(flatten)]
    check: CheckRef,
}

impl PauseArgs {
    /// Pauses the check and prints it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown check; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_paused(&self.check, true, invocation).await
    }
}

impl ResumeArgs {
    /// Resumes the check and prints it.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown check; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set_paused(&self.check, false, invocation).await
    }
}

async fn set_paused(
    check: &CheckRef,
    paused: bool,
    invocation: &Invocation,
) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let id = check.resolve(&hub).await?;
    let answer = hub
        .run::<CheckSetPaused>(check_set_paused::Variables { id, paused })
        .await?;
    invocation.output.data(&answer.set_check_paused, |check| {
        let status = state(&enum_name(&check.status), check.paused);
        written(&check.name, &check.id, &enum_name(&check.type_), &status)
    })
}
