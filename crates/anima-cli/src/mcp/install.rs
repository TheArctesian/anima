//! `anima mcp agents`, `anima mcp install` and `anima mcp uninstall`: the entry `anima` in the
//! configuration of each coding agent on this computer (MCP-027 to MCP-038, plan Section 12).
//! The Settings section calls [`listing`] and [`change`], as it does for the permissions.

pub mod agents;
pub mod files;
pub mod jsonc;
pub mod setup;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use agents::{AGENTS, Agent, State, agent};
use setup::{Action, Checked, Here, Step, apply, check_command, state, steps};

/// One agent as the list shows it (MCP-027).
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub id: &'static str,
    pub name: &'static str,
    pub state: State,
    /// What a change would touch, in order; empty for an agent that is not on this computer (MCP-029).
    pub steps: Vec<Step>,
    /// Why the last change of this agent failed, in words with no text of a file.
    pub problem: Option<String>,
}

/// Each known agent with its state and the step `action` would take.
#[must_use]
pub fn listing(here: &Here, anima: &Checked, action: Action) -> Vec<Row> {
    AGENTS
        .iter()
        .map(|agent| row(agent, here, anima.path(), action))
        .collect()
}

fn row(agent: &Agent, here: &Here, anima: &Path, action: Action) -> Row {
    let state = state(agent, here, anima);
    let steps = if state == State::NotInstalled {
        Vec::new()
    } else {
        steps(agent, here, anima, (action, state)).unwrap_or_default()
    };
    Row {
        id: agent.id,
        name: agent.name,
        state,
        steps,
        problem: None,
    }
}

/// Whether `action` has work for an agent in `state` (MCP-028, MCP-036).
const fn wanted(state: State, action: Action, force: bool) -> bool {
    match (action, state) {
        (Action::Install, State::NotSetUp | State::Different)
        | (Action::Uninstall, State::SetUp) => true,
        (Action::Uninstall, State::Different) => force,
        _ => false,
    }
}

/// What a person asked for: the action, the agents by id (all when empty), and whether an entry
/// that starts a different program may be taken out (MCP-036).
#[derive(Clone, Copy, Debug)]
pub struct Asked<'a> {
    pub action: Action,
    pub ids: &'a [String],
    pub force: bool,
}

/// Does `action` for each agent of `ids` (all when empty) that it has work for, and gives each
/// agent as it is afterwards. One refused agent does not stop the others (MCP-034).
#[must_use]
pub fn change(here: &Here, anima: &Checked, asked: &Asked<'_>) -> Vec<Row> {
    let Asked { action, ids, force } = *asked;
    let anima = anima.path();
    AGENTS
        .iter()
        .filter(|agent| ids.is_empty() || ids.iter().any(|id| id == agent.id))
        .map(|agent| {
            let before = state(agent, here, anima);
            let problem = wanted(before, action, force)
                .then(|| apply(agent, here, anima, action).err())
                .flatten();
            Row {
                problem: problem.map(|error| error.to_string()),
                ..row(agent, here, anima, action)
            }
        })
        .collect()
}

/// The agents and the change, as flags.
#[derive(Debug, clap::Args)]
pub struct ChangeArgs {
    /// Only these agents, by the id that `anima mcp agents` shows; all that are on this computer when absent.
    #[arg(long = "agent", value_name = "ID")]
    agents: Vec<String>,
    /// Print each file and each command line that would be touched, and change nothing.
    #[arg(long)]
    dry_run: bool,
    /// The `anima` program that the agents start; the program that runs this command when absent.
    #[arg(long, value_name = "PATH")]
    command: Option<PathBuf>,
    /// For uninstall: also take out an entry `anima` that starts a different program.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, clap::Subcommand)]
pub enum InstallCommand {
    /// List each coding agent this program knows, and whether it starts `anima mcp`.
    Agents,
    /// Put the entry `anima` into each coding agent on this computer.
    Install(ChangeArgs),
    /// Take the entry `anima` out of each coding agent again.
    Uninstall(ChangeArgs),
}

fn usage(message: String) -> Failure {
    Failure::Usage {
        message,
        hint: None,
    }
}

/// This computer, from the process: the home folder, `PATH` and the user of the home folder.
fn here() -> Result<Here, Failure> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or_else(|| usage("The user home is not set to a folder.".to_owned()))?;
    let path = std::env::var_os("PATH")
        .map_or_else(Vec::new, |path| std::env::split_paths(&path).collect());
    Here::for_this_user(home, path).ok_or_else(|| {
        usage("The home folder does not belong to the user that runs this program.".to_owned())
    })
}

fn anima(command: Option<PathBuf>, here: &Here) -> Result<Checked, Failure> {
    let anima = match command {
        Some(path) => path,
        None => std::env::current_exe().map_err(|_| {
            usage("The path of this program is not known; give it with --command.".to_owned())
        })?,
    };
    check_command(&anima, here.user)
        .map_err(|error| usage(format!("{error}. Give the installed anima with --command.")))
}

fn touched(steps: &[Step]) -> String {
    let words: Vec<String> = steps
        .iter()
        .map(|step| match step {
            Step::Write {
                file,
                copy: Some(copy),
            } => format!("writes {}, keeps {}", file.display(), copy.display()),
            Step::Write { file, copy: None } => format!("writes {}", file.display()),
            Step::Run { program, arguments } => {
                format!("runs {} {}", program.display(), arguments.join(" "))
            }
        })
        .collect();
    words.join("; then ")
}

fn table(rows: &Vec<Row>) -> Table {
    let mut table = Table::new(vec!["ID", "AGENT", "STATE", "A CHANGE"]);
    for row in rows {
        let state = row.problem.as_deref().map_or_else(
            || row.state.words().to_owned(),
            |problem| format!("not changed: {problem}"),
        );
        table.row(vec![
            row.id.into(),
            row.name.into(),
            state.as_str().into(),
            touched(&row.steps).as_str().into(),
        ]);
    }
    table
}

impl InstallCommand {
    /// Lists the agents, or sets them up, or takes the entry out.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an unknown agent, a path of `anima` that MCP-033 refuses or a missing
    /// home folder; [`Failure::Declined`] when the person does not confirm; [`Failure::Conflict`]
    /// when an agent was refused (MCP-034).
    pub fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let here = here()?;
        let (args, action) = match self {
            Self::Agents => {
                let anima = anima(None, &here)?;
                return invocation
                    .output
                    .data(&listing(&here, &anima, Action::Install), table);
            }
            Self::Install(args) => (args, Action::Install),
            Self::Uninstall(args) => (args, Action::Uninstall),
        };
        if let Some(unknown) = args.agents.iter().find(|id| agent(id).is_none()) {
            return Err(usage(format!(
                "No agent has the id {unknown}. `anima mcp agents` lists them."
            )));
        }
        let anima = anima(args.command, &here)?;
        let chosen =
            |row: &Row| args.agents.is_empty() || args.agents.iter().any(|id| id == row.id);
        let planned: Vec<Row> = listing(&here, &anima, action)
            .into_iter()
            .filter(|row| chosen(row) && wanted(row.state, action, args.force))
            .collect();
        if args.dry_run || planned.is_empty() {
            invocation.output.data(&planned, table)?;
            invocation.output.note(if planned.is_empty() {
                "Nothing to change."
            } else {
                "Nothing was changed: this was a dry run."
            });
            return Ok(());
        }
        invocation.output.note(&table(&planned).render(false));
        invocation.prompter.confirm(if action == Action::Install {
            "Set up these agents?"
        } else {
            "Take anima out of these agents?"
        })?;
        let done = change(
            &here,
            &anima,
            &Asked {
                action,
                ids: &args.agents,
                force: args.force,
            },
        );
        invocation.output.data(&done, table)?;
        match done.iter().filter(|row| row.problem.is_some()).count() {
            0 => Ok(()),
            refused => Err(Failure::Conflict {
                message: format!("{refused} agent(s) were not changed; the table says why."),
            }),
        }
    }
}
