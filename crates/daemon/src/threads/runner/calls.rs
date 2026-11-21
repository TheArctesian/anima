//! The calls of the clients of this computer to the runner (TH12, TH13, TH15): each is checked
//! here, written to the log of the runner (THR-044), and answered on the socket.

use agent_threads::control::{Action, Call, Outcome, Refusal, Reply, RunState, ThreadRow};
use agent_threads::drive::{Answer, Effort, Model, Program};
use uuid::Uuid;

use super::codex::Codex;
use super::log::{Kind, Line};
use super::socket::{self, CLIENT_BYTES, CLIENT_QUEUE, Caller, Client, Incoming};
use super::start::NewThread;
use super::thread::{Step, Thread};
use super::{Pending, Runner, Waits};
use crate::threads::wire;

/// What the runner can see of the source of a call: the local socket (THR-044).
const SOURCE: &str = "local socket";
/// The most clients that watch at one time.
const WATCHERS: usize = 16;

/// The Codex calls of a thread of Codex.
pub(super) fn codex_of(row: &ThreadRow, effort: Option<Effort>) -> Option<Codex> {
    (row.program == Program::Codex).then(|| {
        let model = row.model.as_deref().and_then(|name| Model::new(name).ok());
        Codex::new(row.folder.clone(), row.mode, model, effort)
    })
}

impl Runner {
    pub(super) async fn call(&mut self, incoming: Incoming) {
        let Incoming {
            call: Call { id, action },
            client,
        } = incoming;
        let back = Caller {
            call: id,
            client: client.clone(),
        };
        let named = thread_of(&action);
        let answer = match action {
            Action::List => Outcome::Threads {
                threads: self
                    .threads
                    .values()
                    .map(|thread| thread.row.clone())
                    .collect(),
            },
            Action::Watch => {
                self.watch(id, client);
                return;
            }
            Action::Interrupt { thread } => self.interrupt(thread).await,
            Action::Folder { folder } => {
                match super::start::chosen_folder(&folder, self.dir.parent().unwrap_or(&self.dir)) {
                    Ok(real) => Outcome::Folder {
                        notice: crate::threads::notice::of_folder(&real),
                    },
                    Err(why) => Outcome::Refused { why },
                }
            }
            Action::RaiseLimit => {
                self.limits = self.limits.raised();
                self.save();
                Outcome::Limits {
                    limits: self.limits,
                }
            }
            waits => match self.held_call(waits, back.clone()).await {
                Ok(()) => return,
                Err(why) => Outcome::Refused { why },
            },
        };
        if let Outcome::Refused { why } = &answer {
            self.note(Kind::Refusal, named, refusal_word(why));
        }
        back.answer(answer);
    }

    /// A call whose answer waits for the holder: a start, a prompt, an answer or a stop.
    async fn held_call(&mut self, action: Action, back: Caller) -> Result<(), Refusal> {
        match action {
            Action::Send { thread, text } => self.send(thread, text, back).await,
            Action::Answer {
                thread,
                request,
                answer,
            } => {
                self.answer(Answering { thread, request }, &answer, back)
                    .await
            }
            Action::Stop { thread } => self.stop(thread, back).await,
            start => match NewThread::of(start) {
                Some(new) => self.start(new, back).await,
                None => Err(Refusal::BadValue),
            },
        }
    }

    /// A client that watches: the newest updates the runner keeps first, as many of each thread
    /// as the queue of the client holds, then each new one.
    fn watch(&mut self, id: u64, client: Client) {
        let threads = self.threads.len().max(1);
        let (share, bytes) = ((CLIENT_QUEUE - 64) / threads, (CLIENT_BYTES / 2) / threads);
        let mut told = socket::tell(
            &client,
            &Reply::Done {
                id,
                result: Outcome::Ok,
            },
        );
        for thread in self.threads.values() {
            for update in thread.kept_newest(share, bytes) {
                told = told && socket::tell(&client, &Reply::Update(Box::new((*update).clone())));
            }
        }
        // This watch alone gets the row and the waiting requests of each thread after the kept
        // updates, so that a request whose update is older than its share still has a card. The
        // other watches already have them (THR-030).
        for thread in self.threads.values_mut() {
            for change in thread.now_changes() {
                let update = thread.number(change);
                told = told && socket::tell(&client, &Reply::Update(Box::new(update)));
            }
        }
        if told {
            // The oldest watch gives way and is closed, so that the watches of this user stay
            // few and the client of that watch knows it (THR-045).
            if self.watchers.len() >= WATCHERS {
                self.watchers.remove(0).close();
            }
            self.watchers.push(client);
        }
    }

    pub(super) fn note(&self, kind: Kind, thread: Option<Uuid>, what: &str) {
        let line = Line {
            at: chrono::Utc::now(),
            kind,
            thread,
            source: SOURCE,
            what,
        };
        if let Err(error) = self.log.write(&line) {
            tracing::warn!(%error, "the log of the runner was not written");
        }
    }

    /// A prompt of the person. The client gets its answer when the holder took the line.
    async fn send(&mut self, id: Uuid, text: String, back: Caller) -> Result<(), Refusal> {
        let thread = self.threads.get_mut(&id).ok_or(Refusal::UnknownThread)?;
        let lines = thread.send(&text)?;
        let steps = lines.into_iter().map(Step::Write).collect();
        match self.apply(id, steps).await {
            Err(_) => Err(Refusal::Holder),
            Ok(Some(command)) => {
                let waits = Waits::Send {
                    thread: id,
                    back,
                    text,
                };
                self.waits.insert(command, waits);
                Ok(())
            }
            // A Codex prompt that waits in the thread for its ID: it is the thread's now.
            Ok(None) => {
                self.note(Kind::Prompt, Some(id), "sent");
                let _ = self.apply(id, Thread::sent(&text)).await;
                back.answer(Outcome::Ok);
                Ok(())
            }
        }
    }

    /// An answer of the person. The request closes, and the client gets its answer, when the
    /// holder took the line (THR-030).
    async fn answer(
        &mut self,
        to: Answering,
        answer: &Answer,
        back: Caller,
    ) -> Result<(), Refusal> {
        let Answering {
            thread: id,
            request,
        } = to;
        let thread = self.threads.get_mut(&id).ok_or(Refusal::UnknownThread)?;
        let line = thread.answer(request, answer)?;
        // The record says that the answer went, before the line goes (THR-030).
        self.save();
        let Ok(Some(command)) = self.apply(id, vec![Step::Write(line)]).await else {
            if let Some(thread) = self.threads.get_mut(&id) {
                thread.answer_failed(request);
            }
            // The record must not say that the answer went: a restart would close the request.
            self.save();
            return Err(Refusal::Holder);
        };
        let pending = Pending {
            thread: id,
            request,
            word: answer_word(answer),
            back,
        };
        self.waits.insert(command, Waits::Answer(pending));
        Ok(())
    }

    async fn interrupt(&mut self, id: Uuid) -> Outcome {
        let Some(thread) = self.threads.get_mut(&id) else {
            return Outcome::Refused {
                why: Refusal::UnknownThread,
            };
        };
        if thread.row.state == RunState::Stopped {
            return Outcome::Refused {
                why: Refusal::NotRunning,
            };
        }
        let steps = thread.interrupt();
        if self.apply(id, steps).await.is_err() {
            return Outcome::Refused {
                why: Refusal::Holder,
            };
        }
        self.save();
        self.note(Kind::Stop, Some(id), "turn stopped");
        Outcome::Ok
    }

    async fn stop(&mut self, id: Uuid, back: Caller) -> Result<(), Refusal> {
        let thread = self.threads.get(&id).ok_or(Refusal::UnknownThread)?;
        if thread.row.state == RunState::Stopped {
            return Err(Refusal::NotRunning);
        }
        let command = self
            .command(wire::Action::Stop {
                thread: id.to_string(),
            })
            .await
            .ok_or(Refusal::Holder)?;
        self.note(Kind::Stop, Some(id), "program stopped");
        self.waits.insert(command, Waits::Stop { back });
        Ok(())
    }
}

/// The request of a thread that an answer is for.
struct Answering {
    thread: Uuid,
    request: Uuid,
}

/// The thread a call names, for the line of the log of a refusal (THR-044).
const fn thread_of(action: &Action) -> Option<Uuid> {
    match action {
        Action::Send { thread, .. }
        | Action::Answer { thread, .. }
        | Action::Interrupt { thread }
        | Action::Stop { thread } => Some(*thread),
        _ => None,
    }
}

const fn answer_word(answer: &Answer) -> &'static str {
    match answer {
        Answer::Allow => "allow",
        Answer::AllowForSession { .. } => "allow for the session",
        Answer::Deny { .. } => "deny",
        Answer::Choose { .. } => "choose",
    }
}

const fn refusal_word(why: &Refusal) -> &'static str {
    match why {
        Refusal::UnknownThread => "unknown thread",
        Refusal::FolderNotPermitted => "folder not permitted",
        Refusal::ProgramNotFound => "program not found",
        Refusal::BadValue => "bad value",
        Refusal::RequestGone => "request gone",
        Refusal::AnswerDoesNotFit => "answer does not fit",
        Refusal::NotRunning => "not running",
        Refusal::Holder => "holder",
        Refusal::OverLimit { .. } => "over the soft limit",
    }
}
