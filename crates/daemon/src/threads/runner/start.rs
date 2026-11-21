//! The start of a new thread (TH11, THR-041, THR-114): the checks of the runner, the count
//! against the soft limit, the start in the holder and its answer.

use std::path::{Path, PathBuf};

use agent_threads::control::{Refusal, RunState, ThreadRow};
use agent_threads::drive::{Effort, Launch, Mode, Model, Program, SessionStart};
use uuid::Uuid;

use super::calls::codex_of;
use super::socket::Caller;
use super::thread::Thread;
use super::{Runner, Waits, login};
use crate::threads::wire;

/// A new thread as the client named it.
pub(super) struct NewThread {
    pub program: Program,
    pub folder: PathBuf,
    pub mode: Mode,
    pub model: Option<String>,
    pub effort: Option<Effort>,
    pub prompt: String,
}

impl NewThread {
    /// The new thread of a start call; `None` for a different call.
    pub(super) fn of(action: agent_threads::control::Action) -> Option<Self> {
        match action {
            agent_threads::control::Action::Start {
                program,
                folder,
                mode,
                model,
                effort,
                prompt,
            } => Some(Self {
                program,
                folder,
                mode,
                model,
                effort,
                prompt,
            }),
            _ => None,
        }
    }
}

impl Runner {
    /// Checks a new thread, counts it against the soft limit (THR-114) and asks the holder to
    /// start its program; the answer goes to the client when the holder answers.
    pub(super) async fn start(&mut self, new: NewThread, back: Caller) -> Result<(), Refusal> {
        if !self.holder_fits {
            return Err(Refusal::Holder);
        }
        if self.running() >= self.limits.running {
            return Err(Refusal::OverLimit {
                limits: self.limits,
            });
        }
        let id = Uuid::now_v7();
        let (row, action) = self.prepared(&new, id)?;
        let codex = codex_of(&row, new.effort);
        self.threads.insert(id, Thread::new(row, new.effort, codex));
        let Some(command) = self.command(action).await else {
            self.threads.remove(&id);
            return Err(Refusal::Holder);
        };
        let prompt = new.prompt;
        self.waits.insert(
            command,
            Waits::Start {
                thread: id,
                back,
                prompt,
            },
        );
        Ok(())
    }

    /// The row of a new thread and the start for the holder: the folder that the person chose
    /// (THR-041), the program on the `PATH` of the login, the arguments of the closed list
    /// (THR-040) and the environment of the login (R-NEW-threadsdrive).
    fn prepared(&self, new: &NewThread, id: Uuid) -> Result<(ThreadRow, wire::Action), Refusal> {
        let folder = chosen_folder(&new.folder, self.dir.parent().unwrap_or(&self.dir))?;
        let model = new
            .model
            .as_deref()
            .map(Model::new)
            .transpose()
            .map_err(|_| Refusal::BadValue)?;
        let path = login::find(new.program, &self.env, login::executable)
            .ok_or(Refusal::ProgramNotFound)?;
        let launch = Launch {
            program: new.program,
            path,
            mode: new.mode,
            model: model.clone(),
            effort: new.effort,
            session: SessionStart::New(id),
        };
        let invocation = launch.invocation().map_err(|_| Refusal::BadValue)?;
        let mut env = self.env.clone();
        if new.program == Program::ClaudeCode {
            env.extend(
                login::CLAUDE_VARIABLES.map(|(name, value)| (name.to_owned(), value.to_owned())),
            );
        }
        let row = ThreadRow {
            id,
            program: new.program,
            folder: folder.clone(),
            mode: new.mode,
            model: model.as_ref().map(|model| model.as_str().to_owned()),
            state: RunState::Starting,
            session: None,
            title: String::new(),
            started: chrono::Utc::now(),
            ended: None,
            open: Vec::new(),
        };
        let action = wire::Action::Start {
            thread: id.to_string(),
            program: invocation.program.to_string_lossy().into_owned(),
            args: invocation.args,
            env,
            folder: folder.to_string_lossy().into_owned(),
        };
        Ok((row, action))
    }
}

/// An absolute folder of this computer that exists, as its real path, and not in the folder of
/// the daemon, which holds its keys and the record, the log and the sockets of the runner
/// (THR-040, THR-041).
fn chosen_folder(folder: &Path, daemon: &Path) -> Result<PathBuf, Refusal> {
    if !folder.is_absolute() {
        return Err(Refusal::FolderNotPermitted);
    }
    let real = std::fs::canonicalize(folder).map_err(|_| Refusal::FolderNotPermitted)?;
    let inside = std::fs::canonicalize(daemon).is_ok_and(|daemon| real.starts_with(daemon));
    if real.is_dir() && !inside {
        Ok(real)
    } else {
        Err(Refusal::FolderNotPermitted)
    }
}
