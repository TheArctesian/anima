//! The program's own nouns: who is signed in, which hub, raw access and completion.

use clap::CommandFactory as _;

use super::Cli;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{api, auth, contexts, mcp};

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
        }
    }
}
