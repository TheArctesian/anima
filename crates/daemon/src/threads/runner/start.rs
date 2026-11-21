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

/// The shortest time between two asks of the login shell after a program was not found.
const ASK_AGAIN: std::time::Duration = std::time::Duration::from_secs(60);
use crate::threads::wire;

/// The starts whose copy is being asked for its version (TH97).
pub(super) struct Asks {
    pub sender: tokio::sync::mpsc::Sender<(u64, super::silence::Answers)>,
    pub receiver: Option<tokio::sync::mpsc::Receiver<(u64, super::silence::Answers)>>,
    open: std::collections::BTreeMap<u64, Asking>,
    next: u64,
}

impl Default for Asks {
    fn default() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(16);
        Self {
            sender,
            receiver: Some(receiver),
            open: std::collections::BTreeMap::new(),
            next: 0,
        }
    }
}

/// A start that waits for the answer of its copy.
struct Asking {
    new: NewThread,
    back: Caller,
    path: PathBuf,
    copies: Vec<PathBuf>,
}

/// A new thread as the client named it.
pub(super) struct NewThread {
    pub program: Program,
    pub folder: PathBuf,
    pub mode: Mode,
    pub model: Option<String>,
    pub effort: Option<Effort>,
    pub prompt: String,
    pub program_path: Option<PathBuf>,
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
                program_path,
            } => Some(Self {
                program,
                folder,
                mode,
                model,
                effort,
                prompt,
                program_path,
            }),
            _ => None,
        }
    }
}

impl Runner {
    /// Checks a new thread, counts it against the soft limit (THR-114) and asks the holder to
    /// start its program; the answer goes to the client when the holder answers.
    pub(super) async fn start(&mut self, new: NewThread, back: Caller) -> Result<(), Refusal> {
        // A start whose copy is being asked counts against the soft limit too.
        self.may_start(u32::try_from(self.asks.open.len()).unwrap_or(u32::MAX))?;
        chosen_folder(&new.folder, self.dir.parent().unwrap_or(&self.dir))?;
        if login::find(new.program, &self.env, login::executable, login::trusted).is_none() {
            self.ask_login_again(new.program).await;
        }
        let copies = login::copies(new.program, &self.env, login::executable, login::trusted);
        let path = login::chosen_copy(new.program_path.as_deref(), &copies)?;
        match self.silence.kept(&path) {
            Some(true) => {
                let asking = Asking {
                    new,
                    back,
                    path,
                    copies,
                };
                self.start_checked(asking).await
            }
            Some(false) => Err(self.not_answered(new.program, &path, &copies)),
            None => {
                // The ask runs away from the loop; the start goes on when it answers (TH97).
                // One ask for each copy at a time: a later start waits for the same answer.
                let asked = self.asks.open.values().any(|open| open.path == path);
                let env = login::with_folder_of(self.env.clone(), &path);
                let ask = super::silence::ask(path.clone(), copies.clone(), env);
                let sender = self.asks.sender.clone();
                let id = self.asks.next;
                self.asks.next += 1;
                self.asks.open.insert(
                    id,
                    Asking {
                        new,
                        back,
                        path,
                        copies,
                    },
                );
                if !asked {
                    tokio::spawn(async move {
                        // A runner that ended does not need the answer.
                        let _ = sender.send((id, ask.await)).await;
                    });
                }
                Ok(())
            }
        }
    }

    /// The answers of the copies for the start `id`: it goes on, or it is refused with words.
    /// Each start that waited for that copy goes on in its order, or is refused with words.
    pub(super) async fn asked(&mut self, id: u64, answers: super::silence::Answers) {
        self.silence.keep(&answers);
        let Some(path) = self.asks.open.get(&id).map(|asking| asking.path.clone()) else {
            return;
        };
        // The answer itself decides: a copy replaced while it was asked keeps this answer.
        let answered = answers
            .iter()
            .any(|(copy, version)| *copy == path && version.is_some());
        let waiting: Vec<u64> = self
            .asks
            .open
            .iter()
            .filter(|(_, asking)| asking.path == path)
            .map(|(id, _)| *id)
            .collect();
        for id in waiting {
            let Some(asking) = self.asks.open.remove(&id) else {
                continue;
            };
            let back = asking.back.clone();
            let refusal = if answered {
                self.start_checked(asking).await.err()
            } else {
                Some(self.not_answered(asking.new.program, &asking.path, &asking.copies))
            };
            if let Some(why) = refusal {
                self.note(
                    super::log::Kind::Refusal,
                    None,
                    super::calls::refusal_word(&why),
                );
                back.answer(super::holder_events::refused(why));
            }
        }
    }

    /// The threads whose program has not stopped, which count against the soft limit.
    pub(super) fn running(&self) -> u32 {
        let count = self
            .threads
            .values()
            .filter(|thread| thread.row.state != RunState::Stopped)
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// Whether a new thread may start now: a holder that fits (THR-113), and fewer than the soft
    /// limit of running threads with the `asking` starts that wait for an answer (THR-114).
    fn may_start(&self, asking: u32) -> Result<(), Refusal> {
        if !self.holder.fits || self.holder.quitting {
            return match self.holder_words() {
                Some(said) => Err(Refusal::HolderOlder { said }),
                None => Err(Refusal::Holder),
            };
        }
        if self.running().saturating_add(asking) >= self.limits.running {
            return Err(Refusal::OverLimit {
                limits: self.limits,
            });
        }
        Ok(())
    }

    /// The refusal of a start whose copy did not answer, with the other copies that did.
    fn not_answered(&self, program: Program, path: &Path, copies: &[PathBuf]) -> Refusal {
        let others = self.silence.answered_others(path, copies);
        let said = super::silence::not_answered(program, path, &others);
        let answered = others.into_iter().map(|(copy, _)| copy).collect();
        Refusal::ProgramDoesNotAnswer { said, answered }
    }

    /// The start of a thread whose copy answered its version.
    async fn start_checked(&mut self, asking: Asking) -> Result<(), Refusal> {
        let Asking {
            new,
            back,
            path,
            copies,
        } = asking;
        // The holder or the count can change while the copy is asked.
        self.may_start(0)?;
        let id = Uuid::now_v7();
        let (row, action) = self.prepared(&new, id, &path)?;
        let mut notice = crate::threads::notice::of_folder(&row.folder);
        let named = new.program_path.is_some();
        notice.extend(super::silence::chosen(new.program, &path, named, &copies));
        self.silence.started(id, new.program, path);
        let codex = codex_of(&row, new.effort);
        self.threads.insert(id, Thread::new(row, new.effort, codex));
        if !notice.is_empty() {
            // The thread keeps the notice as its first line (THR-046).
            let text = notice.join("\n");
            let event = agent_threads::drive::DriveEvent::Notice { text };
            self.publish(id, agent_threads::control::Change::Said { event });
        }
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

    /// Ends each thread that is still starting after [`super::silence::FIRST_ANSWER`]; its end
    /// says which program at which path said nothing (TH97).
    pub(super) async fn end_the_silent(&mut self) {
        let threads: Vec<_> = self
            .threads
            .values()
            .map(|thread| (thread.row.id, thread.row.state, thread.row.started))
            .collect();
        for id in self.silence.overdue(&threads, chrono::Utc::now()) {
            let stop = wire::Action::Stop {
                thread: id.to_string(),
            };
            if self.command(stop).await.is_some() {
                self.note(super::log::Kind::Stop, Some(id), "no first answer");
            }
        }
    }

    /// Asks the login shell again for its environment, at most once in [`ASK_AGAIN`]: it may not
    /// have answered when the daemon started on a busy computer. The answer is kept only when it
    /// finds the program, so that a shell that does not answer does not take a good environment.
    async fn ask_login_again(&mut self, program: Program) {
        if self.asked_again.is_some_and(|at| at.elapsed() < ASK_AGAIN) {
            return;
        }
        self.asked_again = Some(std::time::Instant::now());
        let env = login::environment(&self.daemon_env).await;
        if login::find(program, &env, login::executable, login::trusted).is_some() {
            self.env = env;
        }
    }

    /// The row of a new thread and the start for the holder: the folder that the person chose
    /// (THR-041), the program on the `PATH` of the login, the arguments of the closed list
    /// (THR-040) and the environment of the login (R-308).
    fn prepared(
        &self,
        new: &NewThread,
        id: Uuid,
        path: &Path,
    ) -> Result<(ThreadRow, wire::Action), Refusal> {
        let folder = chosen_folder(&new.folder, self.dir.parent().unwrap_or(&self.dir))?;
        let model = new
            .model
            .as_deref()
            .map(Model::new)
            .transpose()
            .map_err(|_| Refusal::BadValue)?;
        let launch = Launch {
            program: new.program,
            path: path.to_owned(),
            mode: new.mode,
            model: model.clone(),
            effort: new.effort,
            session: SessionStart::New(id),
        };
        let invocation = launch.invocation().map_err(|_| Refusal::BadValue)?;
        let mut env = login::with_folder_of(self.env.clone(), path);
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
pub(super) fn chosen_folder(folder: &Path, daemon: &Path) -> Result<PathBuf, Refusal> {
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
