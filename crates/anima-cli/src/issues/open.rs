//! `anima issue close` and `anima issue reopen`: close or open an issue on GitHub through the
//! hub (OD-E14, PRJ-028). Nothing changes when it is already so.

use super::documents::{IssueSetOpen, issue_set_open};
use super::render;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Args)]
pub struct CloseArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The issue's number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    number: i64,
    /// Why it is closed.
    #[arg(long, value_enum)]
    reason: Option<ReasonArg>,
}

#[derive(Debug, clap::Args)]
pub struct ReopenArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The issue's number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    number: i64,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ReasonArg {
    Completed,
    NotPlanned,
}

impl CloseArgs {
    /// Closes the issue and prints it or, with `--json`, the K1 `Issue` without its body.
    ///
    /// # Errors
    /// The hub's refusal (`VALIDATION` for the number of a pull request).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let reason = self.reason.map(|reason| match reason {
            ReasonArg::Completed => issue_set_open::IssueCloseReason::COMPLETED,
            ReasonArg::NotPlanned => issue_set_open::IssueCloseReason::NOT_PLANNED,
        });
        set(invocation, &self.repository, self.number, (false, reason)).await
    }
}

impl ReopenArgs {
    /// Opens the issue and prints it or, with `--json`, the K1 `Issue` without its body.
    ///
    /// # Errors
    /// The hub's refusal (`VALIDATION` for the number of a pull request).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        set(invocation, &self.repository, self.number, (true, None)).await
    }
}

/// Sends `setIssueOpen` with `open` and the reason for closing.
async fn set(
    invocation: &Invocation,
    repository: &str,
    number: i64,
    (open, reason): (bool, Option<issue_set_open::IssueCloseReason>),
) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let variables = issue_set_open::Variables {
        repository: resolve(&hub, repository).await?,
        number,
        open,
        reason,
    };
    let issue = hub.run::<IssueSetOpen>(variables).await?.set_issue_open;
    invocation.output.data(&issue, render::changed)
}
