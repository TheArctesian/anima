//! `anima mcp`: an MCP server on standard input and output for coding agents (MCP-001..011).
//! It uses the context and token of every other command, checks the token once with `me`,
//! then offers the tools its permissions and the token allow (MCP-002, MCP-003).

pub mod arguments;
pub mod catalogue;
pub mod definition;
pub mod outcome;
pub mod permissions;
pub mod server;
pub mod tools;
pub mod untrusted;

use crate::auth::documents::{Me, me};
use crate::failure::Failure;
use crate::hub::{Token, TokenKind};
use crate::invocation::Invocation;
use crate::output::Table;
use catalogue::{Grant, ToolSet};
use permissions::{Override, Permissions, every_set_when_empty};
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
    /// Show or save which tool sets are on and which may change things.
    #[command(subcommand)]
    Permissions(PermissionsCommand),
}

#[derive(Debug, clap::Subcommand)]
enum PermissionsCommand {
    /// Show the saved permissions of this hub.
    Show,
    /// Save the permissions of this hub; `anima mcp` uses them at every start.
    Set(SetArgs),
}

/// The flags at start; each overrides the saved value for this run.
#[derive(Debug, Default, clap::Args)]
struct StartFlags {
    /// Offer only these tool sets, comma separated.
    #[arg(long, value_enum, value_delimiter = ',')]
    toolsets: Option<Vec<ToolSet>>,
    /// Offer change tools: in every set, or only in the sets named, comma separated.
    #[arg(long, value_enum, value_delimiter = ',', num_args = 0.., conflicts_with = "read_only")]
    allow_write: Option<Vec<ToolSet>>,
    /// Offer no change tool, whatever is saved.
    #[arg(long)]
    read_only: bool,
}

#[derive(Debug, clap::Args)]
struct SetArgs {
    /// Offer only these tool sets, comma separated.
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        conflicts_with = "all_toolsets"
    )]
    toolsets: Option<Vec<ToolSet>>,
    /// Offer every tool set.
    #[arg(long)]
    all_toolsets: bool,
    /// Permit change tools: in every set, or only in the sets named, comma separated.
    #[arg(long, value_enum, value_delimiter = ',', num_args = 0.., conflicts_with = "read_only")]
    write: Option<Vec<ToolSet>>,
    /// Permit no change tool.
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
            None => start(&self.start, invocation).await,
            Some(McpCommand::Permissions(PermissionsCommand::Show)) => show(invocation),
            Some(McpCommand::Permissions(PermissionsCommand::Set(args))) => args.save(invocation),
        }
    }
}

/// Checks the token with `me` before any protocol output, then serves.
async fn start(flags: &StartFlags, invocation: &Invocation) -> Result<(), Failure> {
    let permissions = saved(invocation)?.overridden(&Override {
        toolsets: flags.toolsets.clone(),
        write: flags.allow_write.clone(),
        read_only: flags.read_only,
    });
    let hub = invocation.hub()?;
    hub.run::<Me>(me::Variables).await?;
    let grant = Grant {
        token: hub.token().map_or(TokenKind::Login, Token::kind),
        toolsets: permissions.toolsets.clone(),
        write: permissions.write.clone(),
    };
    let origin = hub.origin().as_text();
    let server = Server::new(hub, grant, &permissions);
    invocation.output.note(&format!(
        "anima mcp: {} tools for {origin} on standard input and output. {}",
        server.tool_count(),
        permissions.statement()
    ));
    server.run().await
}

/// The saved permissions of the selected hub; none saved is read-only.
fn saved(invocation: &Invocation) -> Result<Permissions, Failure> {
    let origin = invocation.origin()?.as_text();
    match invocation.config()?.mcp.get(&origin) {
        Some(saved) => Permissions::from_saved(saved),
        None => Ok(Permissions::default()),
    }
}

fn show(invocation: &Invocation) -> Result<(), Failure> {
    let permissions = saved(invocation)?;
    print(invocation, &permissions)
}

fn print(invocation: &Invocation, permissions: &Permissions) -> Result<(), Failure> {
    invocation.output.data(permissions, |permissions| {
        let mut table = Table::record();
        table.pair("Tool sets", catalogue::names(&permissions.offered()));
        let writable: Vec<ToolSet> = permissions
            .offered()
            .into_iter()
            .filter(|set| permissions.write.contains(set))
            .collect();
        table.pair("May change", catalogue::names(&writable));
        table.pair("In words", permissions.statement());
        table
    })
}

impl SetArgs {
    fn save(self, invocation: &Invocation) -> Result<(), Failure> {
        let origin = invocation.origin()?.as_text();
        let mut config = invocation.config()?;
        let mut permissions = match config.mcp.get(&origin) {
            Some(saved) => Permissions::from_saved(saved)?,
            None => Permissions::default(),
        };
        if self.all_toolsets {
            permissions.toolsets.clear();
        }
        if let Some(toolsets) = self.toolsets {
            permissions.toolsets = toolsets;
        }
        if let Some(write) = &self.write {
            permissions.write = every_set_when_empty(write);
        }
        if self.read_only {
            permissions.write.clear();
        }
        config.mcp.insert(origin, permissions.to_saved());
        config.save(invocation.config_dir()?)?;
        print(invocation, &permissions)
    }
}
