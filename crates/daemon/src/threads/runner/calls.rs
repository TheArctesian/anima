//! The calls of the clients of this computer to the runner (TH12, TH13, TH15): each is checked
//! here, written to the log of the runner (THR-044), and answered on the socket.

use agent_threads::control::{Action, Call, Outcome, Refusal, Reply, RunState, ThreadRow};
use agent_threads::drive::{Answer, Effort, Model, Program};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::codex::Codex;
use super::log::{Kind, Line};
use super::socket::Incoming;
use super::start::NewThread;
use super::{Runner, Waits};
use crate::threads::wire;

/// What the runner can see of the source of a call: the local socket (THR-044).
const SOURCE: &str = "local socket";

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
            start @ Action::Start { .. } => {
                let Some(new) = NewThread::of(start) else {
                    return;
                };
                match self.start(new, id, client.clone()).await {
                    Ok(()) => return,
                    Err(why) => Outcome::Refused { why },
                }
            }
            Action::Send { thread, text } => self.send(thread, &text).await,
            Action::Answer {
                thread,
                request,
                answer,
            } => self.answer(thread, request, &answer).await,
            Action::Interrupt { thread } => self.interrupt(thread).await,
            Action::Stop { thread } => match self.stop(thread, id, client.clone()).await {
                Ok(()) => return,
                Err(why) => Outcome::Refused { why },
            },
            Action::RaiseLimit => {
                self.limits = self.limits.raised();
                self.save();
                Outcome::Limits {
                    limits: self.limits,
                }
            }
        };
        if let Outcome::Refused { why } = &answer {
            self.note(Kind::Refusal, None, refusal_word(why));
        }
        let _ = client.send(Reply::Done { id, result: answer });
    }

    /// A client that watches: the updates the runner keeps first, then each new one.
    fn watch(&mut self, id: u64, client: mpsc::UnboundedSender<Reply>) {
        let _ = client.send(Reply::Done {
            id,
            result: Outcome::Ok,
        });
        for thread in self.threads.values() {
            for update in thread.kept() {
                let _ = client.send(Reply::Update(Box::new(update.clone())));
            }
        }
        self.watchers.push(client);
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

    async fn send(&mut self, id: Uuid, text: &str) -> Outcome {
        let Some(thread) = self.threads.get_mut(&id) else {
            return Outcome::Refused {
                why: Refusal::UnknownThread,
            };
        };
        match thread.send(text) {
            Ok(steps) => {
                self.apply(id, steps).await;
                self.note(Kind::Prompt, Some(id), "sent");
                Outcome::Ok
            }
            Err(why) => Outcome::Refused { why },
        }
    }

    async fn answer(&mut self, id: Uuid, request: Uuid, answer: &Answer) -> Outcome {
        let Some(thread) = self.threads.get_mut(&id) else {
            return Outcome::Refused {
                why: Refusal::UnknownThread,
            };
        };
        match thread.answer(request, answer) {
            Ok(steps) => {
                self.apply(id, steps).await;
                self.save();
                self.note(Kind::Answer, Some(id), answer_word(answer));
                Outcome::Ok
            }
            Err(why) => Outcome::Refused { why },
        }
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
        self.apply(id, steps).await;
        self.save();
        self.note(Kind::Stop, Some(id), "turn stopped");
        Outcome::Ok
    }

    async fn stop(
        &mut self,
        id: Uuid,
        call: u64,
        client: mpsc::UnboundedSender<Reply>,
    ) -> Result<(), Refusal> {
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
        self.waits.insert(command, Waits::Stop { call, client });
        Ok(())
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
