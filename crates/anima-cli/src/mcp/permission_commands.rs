//! `anima mcp permissions show|set|reset`: each tool's saved state for one hub, by hub origin,
//! in the configuration file (MCP-012, MCP-014). The Settings panel reads `show --json`.

use super::catalogue::{Kind, ToolSet};
use super::permission_change::{Change, Shown, save, saved_in, shown};
use super::permissions::{Permissions, State};
use super::tools::catalogue;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Subcommand)]
pub enum PermissionsCommand {
    /// Show each tool and its saved state for this hub: allow, ask or off.
    Show,
    /// Save tool states for this hub; `anima mcp` uses them at every start.
    Set(SetArgs),
    /// Forget this hub's saved states: read tools allow, change tools off.
    Reset,
}

/// The new states. Bulk choices go first, then `--allow`, `--ask` and `--off`, so a later
/// and safer state wins.
#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("states")
        .required(true)
        .multiple(true)
        .args(["allow", "ask", "off", "allow_all", "ask_all", "off_all", "read_only"])
))]
pub struct SetArgs {
    /// Allow these tools or tool sets, comma separated.
    #[arg(long, value_delimiter = ',', value_name = "TOOLS|SETS")]
    allow: Vec<String>,
    /// Ask the operator before each call of these tools or tool sets.
    #[arg(long, value_delimiter = ',', value_name = "TOOLS|SETS")]
    ask: Vec<String>,
    /// Turn these tools or tool sets off.
    #[arg(long, value_delimiter = ',', value_name = "TOOLS|SETS")]
    off: Vec<String>,
    #[command(flatten)]
    bulk: Bulk,
    /// Only the tools of this set, for --allow-all, --ask-all or --off-all.
    #[arg(long, value_enum, requires = "bulk")]
    toolset: Option<ToolSet>,
    /// Only the tools of this kind, for --allow-all, --ask-all or --off-all.
    #[arg(long, value_enum, requires = "bulk")]
    kind: Option<Kind>,
    /// Turn every change tool off: --off-all --kind change.
    #[arg(long, conflicts_with = "bulk")]
    read_only: bool,
}

/// One state for many tools; `--toolset` and `--kind` narrow it.
#[derive(Debug, clap::Args)]
struct Bulk {
    /// Allow every tool (or those of --toolset and --kind). Unless --kind read, it asks for
    /// confirmation (or --yes): it turns change tools on.
    #[arg(long, group = "bulk")]
    allow_all: bool,
    /// Ask before each call of every tool (or those of --toolset and --kind).
    #[arg(long, group = "bulk")]
    ask_all: bool,
    /// Turn every tool off (or those of --toolset and --kind).
    #[arg(long, group = "bulk")]
    off_all: bool,
}

impl Bulk {
    /// The state chosen for many tools; clap lets only one of the flags through.
    const fn state(&self) -> Option<State> {
        if self.allow_all {
            Some(State::Allow)
        } else if self.ask_all {
            Some(State::Ask)
        } else if self.off_all {
            Some(State::Off)
        } else {
            None
        }
    }
}

impl PermissionsCommand {
    /// Shows, saves or resets this hub's permissions.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an unknown name or an unconfirmed `--allow-all`; [`Failure`] for
    /// an unreadable or unwritable configuration.
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Show => show(invocation, &saved(invocation)?),
            Self::Set(args) => args.save(invocation),
            Self::Reset => {
                let origin = invocation.origin()?;
                let mut permissions = saved(invocation)?;
                Change::Reset.apply(&mut permissions, &catalogue())?;
                save(
                    invocation.config()?,
                    invocation.config_dir()?,
                    &origin,
                    &permissions,
                )?;
                show(invocation, &permissions)
            }
        }
    }
}

/// The saved permissions of the selected hub; nothing saved is the built-in defaults.
///
/// # Errors
/// [`Failure`] for an unreadable configuration or a saved name that does not exist.
pub fn saved(invocation: &Invocation) -> Result<Permissions, Failure> {
    saved_in(&invocation.config()?, &invocation.origin()?)
}

fn show(invocation: &Invocation, permissions: &Permissions) -> Result<(), Failure> {
    let shown = shown(&invocation.origin()?, permissions);
    invocation.output.data(&shown, table)?;
    invocation.output.note(&shown.summary);
    Ok(())
}

fn table(shown: &Shown) -> Table {
    let mut table = Table::new(vec!["TOOL SET", "TOOL", "KIND", "NEEDS SIGN-IN", "STATE"]);
    for tool in &shown.tools {
        let sign_in = if tool.needs_sign_in { "yes" } else { "no" };
        table.row(vec![
            tool.toolset.into(),
            tool.tool.into(),
            tool.kind.name().into(),
            sign_in.into(),
            tool.state.name().into(),
        ]);
    }
    table
}

impl SetArgs {
    fn save(self, invocation: &Invocation) -> Result<(), Failure> {
        let origin = invocation.origin()?;
        let mut permissions = saved(invocation)?;
        let changes = self.changes();
        for change in &changes {
            change.apply(&mut permissions, &catalogue())?;
        }
        if changes.iter().any(Change::allows_changes) {
            let question = "Allow change tools: the agent may then change the hub without asking?";
            if !invocation.prompter.confirm(question)? {
                return Err(Failure::Usage {
                    message: "Nothing was saved: --allow-all was not confirmed.".to_owned(),
                    hint: Some("Add --yes, or narrow it with --kind read.".to_owned()),
                });
            }
        }
        save(
            invocation.config()?,
            invocation.config_dir()?,
            &origin,
            &permissions,
        )?;
        show(invocation, &permissions)
    }

    /// The new states as changes, in order: bulk, allow, ask, off.
    fn changes(&self) -> Vec<Change> {
        let mut changes = Vec::new();
        if self.read_only {
            changes.push(Change::Bulk {
                toolset: None,
                kind: Some(Kind::Change),
                state: State::Off,
            });
        }
        if let Some(state) = self.bulk.state() {
            changes.push(Change::Bulk {
                toolset: self.toolset,
                kind: self.kind,
                state,
            });
        }
        for (names, state) in [
            (&self.allow, State::Allow),
            (&self.ask, State::Ask),
            (&self.off, State::Off),
        ] {
            changes.push(Change::Choose {
                names: names.clone(),
                state,
            });
        }
        changes
    }
}
