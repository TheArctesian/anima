//! `anima agents`: the AI agent programs on the devices; today the use their daemons count
//! (R-135; AIS-020, CMD-030).

pub mod documents;
pub mod report;
pub mod usage;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum AgentCommand {
    /// Sum the sessions, prompts, tool calls and tokens of Claude Code and Codex, with an
    /// estimate of their cost.
    Usage(usage::UsageArgs),
}

impl AgentCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Usage(args) => args.run(invocation).await,
        }
    }
}
