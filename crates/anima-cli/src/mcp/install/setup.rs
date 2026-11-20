//! One agent on this computer: whether it is there, what its entry is, what a change would touch,
//! and the change itself (MCP-027 to MCP-036). This is the edge: it reads files and runs the
//! agents' own programs; `agents.rs` and `jsonc.rs` decide what to write.

use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use super::agents::{Agent, Check, NAME, State, Way, arguments, entry, file};
use super::files::{self, FileError, Root, copy_path};
use super::jsonc::{self, JsoncError};

/// This computer as the install needs it. Tests give a temporary home and their own `PATH`.
#[derive(Clone, Debug)]
pub struct Here {
    pub home: PathBuf,
    /// The folders of `PATH`, in order.
    pub path: Vec<PathBuf>,
    /// The user that runs the command.
    pub user: u32,
    pub macos: bool,
}

impl Here {
    fn root(&self) -> Root<'_> {
        Root {
            home: self.home.as_path(),
            user: self.user,
        }
    }
}

/// Why `anima` cannot be the program of an entry (MCP-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("the path of anima is not absolute")]
    NotAbsolute,
    #[error("the path of anima is not a program that can be run")]
    NotAProgram,
    #[error("a different user can replace the anima program or its folder")]
    Replaceable,
}

/// Checks that `anima` is a fixed program that only this user or the system can replace.
///
/// # Errors
/// [`CommandError`] when it is not absolute, not a runnable regular file, or writable by others.
pub fn check_command(anima: &Path, user: u32) -> Result<(), CommandError> {
    if !anima.is_absolute() {
        return Err(CommandError::NotAbsolute);
    }
    // The program may be a link that an installer made (INS-002); what it points at is checked.
    let program = fs::metadata(anima).map_err(|_| CommandError::NotAProgram)?;
    if !program.is_file() || program.permissions().mode() & 0o111 == 0 {
        return Err(CommandError::NotAProgram);
    }
    let folder = anima
        .parent()
        .and_then(|folder| fs::metadata(folder).ok())
        .ok_or(CommandError::NotAProgram)?;
    let link = fs::symlink_metadata(anima).map_err(|_| CommandError::NotAProgram)?;
    for metadata in [&program, &link, &folder] {
        let owned = metadata.uid() == user || metadata.uid() == 0;
        // A link has no mode of its own on Linux (it reads 0777): its folder decides who replaces it.
        let open = !metadata.file_type().is_symlink() && metadata.permissions().mode() & 0o022 != 0;
        if !owned || open {
            return Err(CommandError::Replaceable);
        }
    }
    Ok(())
}

/// The first program named `name` in the folders of `PATH`, as an absolute path.
fn program(here: &Here, name: &str) -> Option<PathBuf> {
    here.path
        .iter()
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(name))
        .find(|path| {
            fs::metadata(path).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
}

/// Whether the agent is on this computer (MCP-027).
#[must_use]
pub fn present(agent: &Agent, here: &Here) -> bool {
    agent
        .programs
        .iter()
        .any(|name| program(here, name).is_some())
        || agent
            .folders
            .iter()
            .any(|folder| here.home.join(folder).is_dir())
}

fn file_of(agent: &Agent, here: &Here) -> Option<PathBuf> {
    match agent.way {
        Way::File { place, .. } => Some(file(place, &here.home, here.macos, Path::is_file)),
        Way::Own { .. } => None,
    }
}

/// Runs an agent's own program with fixed arguments and no shell (MCP-035); `None` when it is not on `PATH`.
fn run(here: &Here, name: &str, arguments: &[String]) -> Option<std::process::Output> {
    let program = program(here, name)?;
    Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .ok()
}

/// What the agent's entry is now (MCP-027). A file way reads the file; an own way asks the program.
#[must_use]
pub fn state(agent: &Agent, here: &Here, anima: &Path) -> State {
    if !present(agent, here) {
        return State::NotInstalled;
    }
    match agent.way {
        Way::File { keys, shape, .. } => {
            let Some(path) = file_of(agent, here) else {
                return State::Unreadable;
            };
            let Ok(text) = files::read(here.root(), &path) else {
                return State::Unreadable;
            };
            match jsonc::member(text.as_deref().unwrap_or(""), keys, NAME) {
                Err(_) => State::Unreadable,
                Ok(None) => State::NotSetUp,
                // A value with a comment in it does not parse: a person wrote it, so it is not ours.
                Ok(Some(found))
                    if serde_json::from_str::<serde_json::Value>(found)
                        .is_ok_and(|value| value == entry(shape, anima)) =>
                {
                    State::SetUp
                }
                Ok(Some(_)) => State::Different,
            }
        }
        Way::Own { program, check, .. } => {
            let (Check::Get(template) | Check::List(template)) = check;
            let Some(output) = run(here, program, &arguments(template, anima)) else {
                return State::NotSetUp;
            };
            let names_anima =
                String::from_utf8_lossy(&output.stdout).contains(&*anima.to_string_lossy());
            match check {
                Check::Get(_) if !output.status.success() => State::NotSetUp,
                Check::Get(_) | Check::List(_) if names_anima => State::SetUp,
                Check::Get(_) => State::Different,
                // A list names no path for a server of a different program: it cannot be told apart.
                Check::List(_) => State::NotSetUp,
            }
        }
    }
}

/// What a change touches, shown before it is done (MCP-029).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Step {
    /// Writes `file`; `copy` is the copy of the file as it was that this change makes, if any.
    Write {
        file: PathBuf,
        copy: Option<PathBuf>,
    },
    /// Runs `program` with `arguments`.
    Run {
        program: PathBuf,
        arguments: Vec<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Install,
    Uninstall,
}

/// Why one agent was not changed. No variant carries text of a file (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("its file was refused: {0}")]
    File(#[from] FileError),
    #[error("its file was refused: {0}")]
    Format(#[from] JsoncError),
    #[error("its program is not on PATH")]
    NoProgram,
    #[error("its program ended with a failure")]
    ProgramFailed,
}

/// The step that `action` would take for `agent` (MCP-029). It reads only.
///
/// # Errors
/// [`SetupError::NoProgram`] when an own way has no program on `PATH`.
pub fn step(agent: &Agent, here: &Here, anima: &Path, action: Action) -> Result<Step, SetupError> {
    match agent.way {
        Way::File { .. } => {
            let file = file_of(agent, here).ok_or(SetupError::NoProgram)?;
            let copy = (file.exists() && !copy_path(&file).exists()).then(|| copy_path(&file));
            Ok(Step::Write { file, copy })
        }
        Way::Own {
            program: name,
            add,
            remove,
            ..
        } => {
            let template = if action == Action::Install {
                add
            } else {
                remove
            };
            Ok(Step::Run {
                program: program(here, name).ok_or(SetupError::NoProgram)?,
                arguments: arguments(template, anima),
            })
        }
    }
}

/// Does `action` for `agent` (MCP-028, MCP-036).
///
/// # Errors
/// [`SetupError`] when the file is refused or the agent's program fails; nothing is then changed.
pub fn apply(agent: &Agent, here: &Here, anima: &Path, action: Action) -> Result<(), SetupError> {
    match agent.way {
        Way::File { keys, shape, .. } => {
            let path = file_of(agent, here).ok_or(SetupError::NoProgram)?;
            let before = files::read(here.root(), &path)?;
            let text = before.as_deref().unwrap_or("");
            let after = match action {
                Action::Install => {
                    let value = serde_json::to_string_pretty(&entry(shape, anima))
                        .map_err(|_| JsoncError::Malformed)?;
                    Some(jsonc::set(text, keys, NAME, &value)?)
                }
                Action::Uninstall => jsonc::remove(text, keys, NAME)?,
            };
            match after {
                Some(after) if after != text => Ok(files::replace(
                    here.root(),
                    &path,
                    before.as_deref(),
                    &after,
                )?),
                _ => Ok(()),
            }
        }
        Way::Own {
            program,
            add,
            remove,
            ..
        } => {
            // An entry that is there is taken out first: the agents refuse to add a name twice.
            if action == Action::Install {
                run(here, program, &arguments(remove, anima));
            }
            let template = if action == Action::Install {
                add
            } else {
                remove
            };
            let output =
                run(here, program, &arguments(template, anima)).ok_or(SetupError::NoProgram)?;
            if output.status.success() {
                Ok(())
            } else {
                Err(SetupError::ProgramFailed)
            }
        }
    }
}

#[cfg(test)]
mod tests;
