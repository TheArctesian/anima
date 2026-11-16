//! `anima repo status set <repository> <sha>`: writes a commit status under an `anima/`
//! context through the hub (PRO-010, PRJ-028). The hub refuses any other context, so Anima
//! cannot write the status of another system.

use super::documents::{CommitStatusSet, commit_status_set};
use crate::failure::Failure;
use crate::forge::render::text;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::projects::repository::resolve;

#[derive(Debug, clap::Subcommand)]
pub enum StatusCommand {
    /// Write a commit status under an `anima/` context. Needs the operator's sign-in.
    Set(SetArgs),
}

impl StatusCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Set(args) => args.run(invocation).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The commit, 40 hex digits.
    sha: String,
    /// The status.
    #[arg(long, value_enum)]
    state: StatusStateArg,
    /// `anima/` and then 1 to 40 of a-z, 0-9 and -, such as `anima/build`.
    #[arg(long)]
    context: String,
    /// At most 140 characters; the hub cuts a longer one.
    #[arg(long)]
    description: Option<String>,
    /// A link below the hub's public address.
    #[arg(long, value_name = "URL")]
    url: Option<String>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum StatusStateArg {
    Pending,
    Success,
    Failure,
    Error,
}

impl SetArgs {
    /// Prints the status or, with `--json`, the K1 `CommitStatus`.
    ///
    /// # Errors
    /// The hub's refusal (`VALIDATION` for a context outside `anima/` or a link elsewhere).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let state = match self.state {
            StatusStateArg::Pending => commit_status_set::CommitStatusState::PENDING,
            StatusStateArg::Success => commit_status_set::CommitStatusState::SUCCESS,
            StatusStateArg::Failure => commit_status_set::CommitStatusState::FAILURE,
            StatusStateArg::Error => commit_status_set::CommitStatusState::ERROR,
        };
        let variables = commit_status_set::Variables {
            input: commit_status_set::CommitStatusInput {
                repository: resolve(&hub, &self.repository).await?,
                sha: self.sha,
                state,
                context: self.context,
                description: self.description,
                target_url: self.url,
            },
        };
        let status = hub
            .run::<CommitStatusSet>(variables)
            .await?
            .set_commit_status;
        invocation.output.data(&status, |status| {
            let shown = serde_json::to_value(status).unwrap_or_default();
            let mut table = Table::record();
            table.pair("Commit", text(&shown, "sha"));
            table.pair("Context", text(&shown, "context"));
            table.pair("State", text(&shown, "state"));
            table.pair("Description", text(&shown, "description"));
            table.pair("Link", text(&shown, "targetUrl"));
            table
        })
    }
}
