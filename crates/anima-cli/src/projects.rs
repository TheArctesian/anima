//! `anima project`: the projects the hub found in the repositories of a source account, their
//! resources, and which repositories it leaves out (PRJ-002, PRJ-016, PRO-004).

pub mod documents;
pub mod get;
pub mod left_out;
pub mod list;
pub mod render;
pub mod repository;
pub mod resource;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum ProjectCommand {
    /// List the projects, or with `--left-out` the ones with repositories you left out.
    List(list::ListArgs),
    /// Show one project: repositories and resources.
    Get(get::GetArgs),
    /// Leave a repository out: the hub does not read it again until you restore it.
    LeaveOut(left_out::LeaveOutArgs),
    /// Restore a repository that was left out.
    Restore(left_out::RestoreArgs),
    /// A resource found in a project's repositories.
    Resource {
        #[command(subcommand)]
        command: resource::ResourceCommand,
    },
}

impl ProjectCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown project or repository, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::LeaveOut(args) => args.run(invocation).await,
            Self::Restore(args) => args.run(invocation).await,
            Self::Resource { command } => command.run(invocation).await,
        }
    }
}
