//! What `anima pr`, `anima issue`, `anima workflow`, `anima code` and `anima repo` share: the rows a repository on GitHub
//! gives (PRJ-030 to PRJ-035). The hub reads them live; the command line only asks and prints.

pub mod comments;
pub mod render;
pub mod rows;

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{code, issues, pull_requests, repositories, workflows};

/// The nouns that read a repository on GitHub through the hub.
#[derive(Debug, clap::Subcommand)]
pub enum ForgeCommand {
    /// Pull requests of a repository (list, get).
    Pr {
        #[command(subcommand)]
        command: pull_requests::PullRequestCommand,
    },
    /// Issues of a repository (list, get).
    Issue {
        #[command(subcommand)]
        command: issues::IssueCommand,
    },
    /// Workflows of a repository and their runs (list, runs, run).
    Workflow {
        #[command(subcommand)]
        command: workflows::WorkflowCommand,
    },
    /// The code of a repository (ls, show).
    Code {
        #[command(subcommand)]
        command: code::CodeCommand,
    },
    /// The history of a repository on GitHub (commits, branches, releases).
    Repo {
        #[command(subcommand)]
        command: repositories::RepositoryCommand,
    },
}

impl ForgeCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for an unknown repository, pull request, issue or run, or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Pr { command } => command.run(invocation).await,
            Self::Issue { command } => command.run(invocation).await,
            Self::Workflow { command } => command.run(invocation).await,
            Self::Code { command } => command.run(invocation).await,
            Self::Repo { command } => command.run(invocation).await,
        }
    }
}

/// `--state`: which pull requests or issues to list.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum StateArg {
    #[default]
    Open,
    Closed,
    All,
}

impl StateArg {
    /// The word as `--state` takes it, for the next-page hint.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}
