//! `anima mcp`: an MCP server on standard input and output for coding agents (MCP-001..016).
//! It uses the context and token of every other command, checks the token once with `me`,
//! then lists the tools that the saved permissions, the flags and the token allow (MCP-002,
//! MCP-003, MCP-015).

pub mod approval;
pub mod arguments;
pub mod catalogue;
pub mod definition;
pub mod outcome;
pub mod permission_commands;
pub mod permissions;
pub mod selection;
pub mod server;
pub mod tools;
pub mod untrusted;
pub mod widening;

use crate::auth::documents::{Me, me};
use crate::failure::Failure;
use crate::hub::{Token, TokenKind};
use crate::invocation::Invocation;
use catalogue::ToolSet;
use permission_commands::{PermissionsCommand, saved};
use selection::{Flags, select, statement};
use server::Server;

/// `anima mcp` starts the server; `anima mcp permissions` shows or saves what it may do.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct McpArgs {
    #[command(subcommand)]
    command: Option<McpCommand>,
    #[command(flatten)]
    start: StartFlags,
}

#[derive(Debug, clap::Subcommand)]
enum McpCommand {
    /// Show or save each tool's state: allow, ask or off.
    #[command(subcommand)]
    Permissions(PermissionsCommand),
}

/// The flags at start, for this run only. They only narrow the saved states, except that
/// `--allow-write` allows change tools that have no saved state (MCP-015).
#[derive(Debug, Default, clap::Args)]
struct StartFlags {
    /// List only the tools of these sets, comma separated.
    #[arg(long, value_enum, value_delimiter = ',')]
    toolsets: Option<Vec<ToolSet>>,
    /// Allow the change tools with no saved state: in every set, or in the sets named.
    #[arg(long, value_enum, value_delimiter = ',', num_args = 0.., conflicts_with = "read_only")]
    allow_write: Option<Vec<ToolSet>>,
    /// Turn every change tool off, whatever is saved.
    #[arg(long)]
    read_only: bool,
}

impl McpArgs {
    /// Starts the server, or shows or saves the permissions.
    ///
    /// # Errors
    /// [`Failure`] for a missing or refused token, an unreadable configuration, or a failed
    /// connection.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self.command {
            None => start(self.start, invocation).await,
            Some(McpCommand::Permissions(command)) => command.run(invocation),
        }
    }
}

/// Checks the token with `me` before any protocol output, then serves.
async fn start(flags: StartFlags, invocation: &Invocation) -> Result<(), Failure> {
    let permissions = saved(invocation)?;
    let hub = invocation.hub()?;
    hub.run::<Me>(me::Variables).await?;
    let token = hub.token().map_or(TokenKind::Login, Token::kind);
    let flags = Flags {
        toolsets: flags.toolsets,
        allow_write: flags.allow_write,
        read_only: flags.read_only,
    };
    let rows = select(tools::catalogue(), Some(token), &permissions, &flags);
    let picture = statement(&rows);
    let origin = hub.origin().as_text();
    let server = Server::new(hub, token, rows);
    invocation.output.note(&format!(
        "anima mcp: {} tools for {origin} on standard input and output. {picture}",
        server.tool_count(),
    ));
    server.run().await
}
