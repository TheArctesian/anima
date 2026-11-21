//! `anima agents`: the AI agent programs on the devices; today the use their daemons count
//! (R-135; AIS-020, CMD-030) and their single sessions (AIS-025), and the sessions of this computer read as threads (THR-007).

pub mod documents;
#[cfg(unix)]
pub mod drive;
pub mod report;
#[cfg(unix)]
pub mod runner;
pub mod sessions;
pub mod threads;
pub mod usage;
#[cfg(unix)]
pub mod watch_text;

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
    /// Start Claude Code or Codex in a folder of this computer as a new thread, through the
    /// thread runner of the daemon. Full access is the default: the program asks nothing.
    #[cfg(unix)]
    Start(drive::StartArgs),
    /// Send a prompt to a thread that this computer runs.
    #[cfg(unix)]
    Send(drive::SendArgs),
    /// Answer a request of a thread: allow (the default), deny, allow for the session, or
    /// choose the answers of a question.
    #[cfg(unix)]
    Approve(drive::ApproveArgs),
    /// Stop the running turn of a thread; the program stays.
    #[cfg(unix)]
    Interrupt(drive::ThreadIdArgs),
    /// End the program of a thread and each process below it.
    #[cfg(unix)]
    Stop(drive::ThreadIdArgs),
    /// List the threads that the runner of this computer started, with their state.
    #[cfg(unix)]
    Running,
    /// Show the updates of the threads of this computer as they happen.
    #[cfg(unix)]
    Watch(drive::WatchArgs),
    /// Raise the soft limit of threads one step, after the question of the limit.
    #[cfg(unix)]
    RaiseLimit,
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
            #[cfg(unix)]
            Self::Start(args) => args.run(invocation).await,
            #[cfg(unix)]
            Self::Send(args) => args.run(invocation).await,
            #[cfg(unix)]
            Self::Approve(args) => args.run(invocation).await,
            #[cfg(unix)]
            Self::Interrupt(args) => drive::interrupt(args, invocation).await,
            #[cfg(unix)]
            Self::Stop(args) => drive::stop(args, invocation).await,
            #[cfg(unix)]
            Self::Running => drive::running(invocation).await,
            #[cfg(unix)]
            Self::Watch(args) => drive::watch(args, invocation).await,
            #[cfg(unix)]
            Self::RaiseLimit => drive::raise_limit(invocation).await,
        }
    }
}
