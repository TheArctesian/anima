//! `anima agents`: the AI agent programs on the devices; today the use their daemons count
//! (R-135; AIS-020, CMD-030) and their single sessions (AIS-025), and the sessions of this computer read as threads (THR-007).

pub mod documents;
pub mod report;
pub mod sessions;
pub mod threads;
pub mod usage;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum AgentCommand {
    /// Sum the sessions, prompts, tool calls and tokens of Claude Code and Codex, with an
    /// estimate of their cost.
    Usage(usage::UsageArgs),
    /// List the single sessions of Claude Code and Codex that the daemons counted, the newest
    /// use first, with their sums and an estimate of their cost.
    Sessions(sessions::SessionsArgs),
    /// List the sessions of Claude Code and Codex on this computer as threads, the last
    /// changed first. Reads the files of this user; sends nothing to the hub.
    Threads(threads::ThreadsArgs),
    /// Show one thread of this computer: the prompts, the replies, the reasoning and the tool
    /// calls with their results.
    Thread(threads::ThreadArgs),
}

impl AgentCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Usage(args) => args.run(invocation).await,
            Self::Sessions(args) => args.run(invocation).await,
            Self::Threads(args) => args.run(invocation),
            Self::Thread(args) => args.run(invocation),
        }
    }
}
