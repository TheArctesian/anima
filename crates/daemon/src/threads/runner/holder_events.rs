//! What the holder says to the runner: what it holds after a connection, the lines of the
//! programs, their end, the answer to each command, and a program that waits because the holder
//! keeps its limit of output (THR-113).

use agent_threads::control::{Change, Outcome, Refusal, RunState};
use agent_threads::drive::DriveEvent;
use uuid::Uuid;

use super::log::Kind;
use super::thread::Thread;
use super::{Pending, Runner, Waits};
use crate::threads::wire::{
    self, Action, Event, Exit, Held, Line, Outcome as HolderOutcome, Refusal as HolderRefusal,
    Stream,
};

impl Runner {
    pub(super) async fn holder_event(&mut self, event: Event) {
        match event {
            Event::Hello { version, threads } => self.hello(version, threads).await,
            Event::Done { id, outcome } => self.done(id, outcome).await,
            Event::Lines {
                thread,
                first,
                lines,
            } => self.lines(&thread, first, lines).await,
            Event::Waiting { thread } => self.waiting(&thread),
            Event::Exited { thread, exit } => self.exited(&thread, exit).await,
        }
    }

    /// After a connection: a holder of a different version takes no start (THR-113); a thread
    /// that the holder does not hold any more stopped; a program the holder holds that no
    /// thread knows is ended (THR-049); a start whose answer was lost is settled.
    async fn hello(&mut self, version: u32, held: Vec<Held>) {
        self.holder_fits = version == wire::VERSION;
        if !self.holder_fits {
            tracing::error!(
                holder = version,
                runner = wire::VERSION,
                "the holder speaks a different version; no thread starts until it ends"
            );
        }
        self.reconcile(&held).await;
        self.settle_starts().await;
        self.end_unknown(held).await;
        self.save();
    }

    /// Each thread that the holder does not hold, or holds ended, stops.
    async fn reconcile(&mut self, held: &[Held]) {
        let ids: Vec<Uuid> = self.threads.keys().copied().collect();
        for id in ids {
            let found = held.iter().find(|entry| entry.thread == id.to_string());
            let Some(thread) = self
                .threads
                .get_mut(&id)
                .filter(|thread| thread.row.state != RunState::Stopped)
            else {
                continue;
            };
            let steps = match found {
                None => thread.lost(),
                Some(Held {
                    exit: Some(exit), ..
                }) => thread.exited(exit.code, exit.signal),
                Some(_) => continue,
            };
            // The link is new: a command that does not reach it is found by the loop.
            let _ = self.apply(id, steps).await;
        }
    }

    /// Each program the holder holds that no running thread knows ends, or is forgotten when
    /// it ended (THR-049).
    async fn end_unknown(&mut self, held: Vec<Held>) {
        for entry in held {
            let known = Uuid::parse_str(&entry.thread)
                .ok()
                .and_then(|id| self.threads.get(&id))
                .is_some_and(|thread| thread.row.state != RunState::Stopped);
            if known {
                continue;
            }
            let thread = entry.thread;
            let action = if entry.exit.is_some() {
                Action::Forget { thread }
            } else {
                Action::Stop { thread }
            };
            self.command(action).await;
        }
    }

    /// The starts whose answer the old link lost: a program that the holder holds began; one
    /// that it does not hold is refused.
    async fn settle_starts(&mut self) {
        let starts: Vec<u64> = self
            .waits
            .iter()
            .filter(|(_, waits)| matches!(waits, Waits::Start { .. }))
            .map(|(command, _)| *command)
            .collect();
        for command in starts {
            let Some(Waits::Start {
                thread,
                back,
                prompt,
            }) = self.waits.remove(&command)
            else {
                continue;
            };
            let running = self
                .threads
                .get(&thread)
                .is_some_and(|thread| thread.row.state != RunState::Stopped);
            let result = if running {
                self.began(thread, &prompt).await
            } else {
                self.threads.remove(&thread);
                refused(Refusal::Holder)
            };
            back.answer(result);
        }
    }

    /// The program of a new thread runs: its first prompt goes to it.
    async fn began(&mut self, id: Uuid, prompt: &str) -> Outcome {
        let Some(thread) = self.threads.get_mut(&id) else {
            return refused(Refusal::Holder);
        };
        if thread.row.state == RunState::Stopped {
            return refused(Refusal::NotRunning);
        }
        let steps = thread.begin(prompt);
        self.note(Kind::Start, Some(id), "started");
        let _ = self.apply(id, steps).await;
        self.save();
        Outcome::Started { thread: id }
    }

    async fn lines(&mut self, key: &str, first: u64, lines: Vec<Line>) {
        let Some(id) = Uuid::parse_str(key)
            .ok()
            .filter(|id| self.threads.contains_key(id))
        else {
            return;
        };
        let before = self.threads.get(&id).map(Thread::record_key);
        let mut last = self.read_up_to.get(&id).copied().unwrap_or(0);
        for (at, line) in lines.into_iter().enumerate() {
            let seq = first + at as u64;
            if seq <= last {
                continue;
            }
            last = seq;
            let Some(thread) = self.threads.get_mut(&id) else {
                return;
            };
            let steps = match line.stream {
                Stream::Out => thread.line(&line.text, &mut Uuid::now_v7),
                Stream::Err => {
                    thread.error_line(&line.text);
                    Vec::new()
                }
            };
            if self.apply(id, steps).await.is_err() {
                break;
            }
        }
        self.read_up_to.insert(id, last);
        // The record holds the open requests before the holder drops their lines (THR-030); a
        // line that changed nothing of the record does not write it again.
        let after = self.threads.get(&id).map(Thread::record_key);
        if after != before {
            self.save();
        }
        self.command(Action::Ack {
            thread: key.to_owned(),
            seq: last,
        })
        .await;
    }

    /// The holder keeps its limit of output of a program that the runner did not read: the
    /// program waits, and the thread says so (THR-113).
    fn waiting(&mut self, key: &str) {
        let Some(id) = Uuid::parse_str(key).ok() else {
            return;
        };
        tracing::warn!(%id, "the holder keeps its limit of output of a thread");
        self.publish(
            id,
            Change::Said {
                event: DriveEvent::Notice {
                    text: "The program waits: the holder keeps its limit of output that the runner did not read.".to_owned(),
                },
            },
        );
    }

    async fn exited(&mut self, key: &str, exit: Exit) {
        let Some(id) = Uuid::parse_str(key).ok() else {
            return;
        };
        let Some(thread) = self.threads.get_mut(&id) else {
            return;
        };
        let steps = thread.exited(exit.code, exit.signal);
        let _ = self.apply(id, steps).await;
        self.save();
        self.command(Action::Forget {
            thread: key.to_owned(),
        })
        .await;
    }

    /// The answer of the holder to a command that a call waits for.
    async fn done(&mut self, command: u64, outcome: HolderOutcome) {
        let sent = self.unanswered.remove(&command);
        let Some(waits) = self.waits.remove(&command) else {
            if let HolderOutcome::Refused { why } = outcome {
                self.not_taken(sent, why);
            }
            return;
        };
        match waits {
            Waits::Start {
                thread,
                back,
                prompt,
            } => back.answer(self.start_done(thread, &prompt, &outcome).await),
            Waits::Stop { back } => back.answer(match outcome {
                HolderOutcome::Stopped { all_ended, .. } => Outcome::Stopped { all_ended },
                HolderOutcome::Refused { .. } => refused(Refusal::Holder),
                _ => Outcome::Ok,
            }),
            Waits::Send { thread, back, text } => {
                back.answer(self.sent(thread, &text, &outcome).await);
            }
            Waits::Answer(pending) => {
                let result = self.answered(&pending, &outcome).await;
                pending.back.answer(result);
            }
        }
    }

    /// The holder refused a command that no call waits for: the log says so, and a line that
    /// did not reach the program is said in its thread.
    fn not_taken(&mut self, sent: Option<wire::Command>, why: HolderRefusal) {
        let Some(wire::Command {
            action: Action::Write { thread, .. },
            ..
        }) = sent
        else {
            tracing::warn!(?why, "the holder refused a command of the runner");
            return;
        };
        let Ok(id) = Uuid::parse_str(&thread) else {
            return;
        };
        tracing::warn!(%id, ?why, "the holder did not write a line to the program");
        self.publish(
            id,
            Change::Said {
                event: DriveEvent::Notice {
                    text: "A line did not reach the program: the holder did not take it."
                        .to_owned(),
                },
            },
        );
    }

    async fn start_done(&mut self, id: Uuid, prompt: &str, outcome: &HolderOutcome) -> Outcome {
        if matches!(outcome, HolderOutcome::Refused { .. }) {
            self.threads.remove(&id);
            refused(Refusal::Holder)
        } else {
            self.began(id, prompt).await
        }
    }

    async fn sent(&mut self, id: Uuid, text: &str, outcome: &HolderOutcome) -> Outcome {
        if let Some(why) = taken(outcome) {
            return refused(why);
        }
        self.note(Kind::Prompt, Some(id), "sent");
        let _ = self.apply(id, Thread::sent(text)).await;
        Outcome::Ok
    }

    async fn answered(&mut self, pending: &Pending, outcome: &HolderOutcome) -> Outcome {
        let Some(thread) = self.threads.get_mut(&pending.thread) else {
            return refused(Refusal::UnknownThread);
        };
        if let Some(why) = taken(outcome) {
            thread.answer_failed(pending.request);
            return refused(why);
        }
        let steps = thread.answered(pending.request);
        self.note(Kind::Answer, Some(pending.thread), pending.word);
        let _ = self.apply(pending.thread, steps).await;
        self.save();
        Outcome::Ok
    }
}

/// Why the holder did not take a line, or `None` when it did.
const fn taken(outcome: &HolderOutcome) -> Option<Refusal> {
    match outcome {
        HolderOutcome::Refused {
            why: HolderRefusal::InputClosed | HolderRefusal::UnknownThread,
        } => Some(Refusal::NotRunning),
        HolderOutcome::Refused { .. } => Some(Refusal::Holder),
        _ => None,
    }
}

pub(super) const fn refused(why: Refusal) -> Outcome {
    Outcome::Refused { why }
}
