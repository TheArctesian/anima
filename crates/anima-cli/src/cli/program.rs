//! The program's own nouns: who is signed in, which hub, raw access, completion, and snapshots
//! of the working tree that use git on this computer rather than the hub.

use clap::CommandFactory as _;

use super::Cli;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{api, auth, contexts, mcp, snapshot};

#[derive(Debug, clap::Subcommand)]
pub enum ProgramCommand {
    /// Sign in and out (login, logout, status).
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    /// Named hubs (list, add, use, remove).
    Context {
        #[command(subcommand)]
        command: contexts::ContextCommand,
    },
    /// Send a GraphQL document and print the hub's answer.
    Api(api::ApiArgs),
    /// An MCP server on standard input and output for coding agents, and its permissions.
    Mcp(mcp::McpArgs),
    /// Print shell completion.
    Completion { shell: clap_complete::Shell },
    /// Scratch refs of the working tree on the forge (push, list, delete); git only, no hub.
    Snapshot {
        #[command(subcommand)]
        command: snapshot::SnapshotCommand,
    },
}

impl ProgramCommand {
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Auth { command } => command.run(invocation).await,
            Self::Context { command } => command.run(invocation),
            Self::Api(args) => args.run(invocation).await,
            Self::Mcp(args) => args.run(invocation).await,
            Self::Completion { shell } => {
                let mut command = Cli::command();
                clap_complete::generate(shell, &mut command, "anima", &mut std::io::stdout());
                Ok(())
            }
            Self::Snapshot { command } => command.run(invocation),
        }
    }
}
