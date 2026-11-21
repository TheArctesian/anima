//! What the holder does for each command of the daemon. A command is done one time: the same
//! ID again gets the first answer and does nothing (H10).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::holder::{Shared, locked};
use super::program::{self, Start};
use super::tree;
use super::wire::{self, Action, Command, Event, Outcome, Refusal, Stream, ThreadKey};

/// The time the program of a thread has after `SIGTERM` before `SIGKILL`.
const GRACE: Duration = Duration::from_secs(3);

/// Does `command` and gives its answer to the daemon.
pub async fn run(shared: &Arc<Mutex<Shared>>, command: Command) -> Outcome {
    // `Hello` is done each time: it is how a daemon that starts again gets the lines it missed.
    let repeatable = matches!(command.action, Action::Hello { .. });
    let known = if repeatable {
        None
    } else {
        locked(shared).done.get(&command.id).cloned()
    };
    let outcome = if let Some(outcome) = known {
        outcome
    } else {
        let outcome = act(shared, command.action).await;
        locked(shared).remember(command.id, outcome.clone());
        outcome
    };
    locked(shared).send(&Event::Done {
        id: command.id,
        outcome: outcome.clone(),
    });
    outcome
}

async fn act(shared: &Arc<Mutex<Shared>>, action: Action) -> Outcome {
    match action {
        Action::Hello { version } => hello(shared, version),
        Action::Start {
            thread,
            program,
            args,
            env,
            folder,
        } => {
            let start = Start {
                program: &program,
                args: &args,
                env: &env,
                folder: &folder,
            };
            start_thread(shared, thread, &start).await
        }
        Action::Write { thread, line } => write(shared, &thread, line),
        Action::CloseInput { thread } => {
            if close_input(shared, &thread) {
                Outcome::Ok
            } else {
                refused(Refusal::UnknownThread)
            }
        }
        Action::Mark { thread } => mark(shared, &thread).await,
        Action::Ack { thread, seq } => {
            let Ok(mut state) = shared.lock() else {
                return refused(Refusal::UnknownThread);
            };
            match state.threads.get_mut(&thread) {
                Some(slot) => {
                    slot.outbox.ack(seq);
                    Outcome::Ok
                }
                None => refused(Refusal::UnknownThread),
            }
        }
        Action::EndChildren { thread } => end(shared, &thread, false).await,
        Action::Stop { thread } => {
            close_input(shared, &thread);
            end(shared, &thread, true).await
        }
        Action::Forget { thread } => forget(shared, &thread),
        Action::Quit => {
            let busy = shared
                .lock()
                .is_ok_and(|state| state.threads.values().any(|slot| slot.exit.is_none()));
            if busy {
                refused(Refusal::Busy)
            } else {
                Outcome::Ok
            }
        }
    }
}

fn refused(why: Refusal) -> Outcome {
    Outcome::Refused { why }
}

/// Answers with the version and the threads, then gives again each line the daemon did not
/// confirm and each end it did not see.
fn hello(shared: &Arc<Mutex<Shared>>, version: u32) -> Outcome {
    let Ok(mut state) = shared.lock() else {
        return Outcome::Ok;
    };
    state.draining = version != wire::VERSION;
    state.send(&Event::Hello {
        version: wire::VERSION,
        threads: state.held(),
    });
    for (thread, slot) in &state.threads {
        for (first, lines) in slot.outbox.pending() {
            state.send(&Event::Lines {
                thread: thread.clone(),
                first,
                lines,
            });
        }
        if slot.waiting {
            state.send(&Event::Waiting {
                thread: thread.clone(),
            });
        }
        if let Some(exit) = slot.exit {
            state.send(&Event::Exited {
                thread: thread.clone(),
                exit,
            });
        }
    }
    Outcome::Ok
}

async fn start_thread(
    shared: &Arc<Mutex<Shared>>,
    thread: ThreadKey,
    start: &Start<'_>,
) -> Outcome {
    {
        let Ok(state) = shared.lock() else {
            return refused(Refusal::StartFailed);
        };
        if state.draining || state.closing {
            return refused(Refusal::Version);
        }
        if state.threads.contains_key(&thread) {
            return refused(Refusal::ThreadExists);
        }
    }
    if !start.fits() {
        return refused(Refusal::BadStart);
    }
    let Ok(mut child) = start.spawn() else {
        return refused(Refusal::StartFailed);
    };
    let Some(pid) = child.id() else {
        return refused(Refusal::StartFailed);
    };
    let started = tree::started(pid).await;
    let input = child.stdin.take().map(program::input);
    let (Some(out), Some(err)) = (child.stdout.take(), child.stderr.take()) else {
        return refused(Refusal::StartFailed);
    };
    if let Ok(mut state) = shared.lock() {
        let slot = state.new_slot(pid, started, input);
        state.threads.insert(thread.clone(), slot);
        state.write_record();
    }
    let readers = [
        tokio::spawn(program::read(
            Arc::clone(shared),
            thread.clone(),
            Stream::Out,
            out,
        )),
        tokio::spawn(program::read(
            Arc::clone(shared),
            thread.clone(),
            Stream::Err,
            err,
        )),
    ];
    tokio::spawn(program::wait(Arc::clone(shared), thread, child, readers));
    Outcome::Started { pid }
}

/// Closes the input of the program of `thread`; false when the holder does not know it.
fn close_input(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey) -> bool {
    let Ok(mut state) = shared.lock() else {
        return false;
    };
    state
        .threads
        .get_mut(thread)
        .map(|slot| slot.input = None)
        .is_some()
}

/// Puts `line` in the queue of the input of the program. It never waits: a program that does
/// not read its input gets [`Refusal::InputFull`], and the other commands go on (THR-113).
fn write(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey, mut line: String) -> Outcome {
    use tokio::sync::mpsc::error::TrySendError;
    if !line.ends_with('\n') {
        line.push('\n');
    }
    let Ok(state) = shared.lock() else {
        return refused(Refusal::UnknownThread);
    };
    let Some(slot) = state.threads.get(thread) else {
        return refused(Refusal::UnknownThread);
    };
    match slot.input.as_ref().map(|input| input.try_send(line)) {
        Some(Ok(())) => Outcome::Ok,
        Some(Err(TrySendError::Full(_))) => refused(Refusal::InputFull),
        Some(Err(TrySendError::Closed(_))) | None => refused(Refusal::InputClosed),
    }
}

/// Keeps the processes below the program now as spared (see [`Action::Mark`]).
async fn mark(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey) -> Outcome {
    let root = shared
        .lock()
        .ok()
        .and_then(|state| state.threads.get(thread).map(|slot| slot.pid));
    let Some(root) = root else {
        return refused(Refusal::UnknownThread);
    };
    let below = tree::table()
        .await
        .map_or_else(tree::Known::new, |table| table.below(root));
    if let Ok(mut state) = shared.lock()
        && let Some(slot) = state.threads.get_mut(thread)
    {
        slot.spared = below;
    }
    Outcome::Ok
}

/// What the holder knows of a program when it ends its processes.
struct Ending {
    root: u32,
    gone: bool,
    spare: tree::Known,
    /// The processes seen below the program that may have left its tree since (a stop only).
    seen: tree::Known,
    left_behind: bool,
}

async fn end(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey, with_root: bool) -> Outcome {
    let found = locked(shared).threads.get(thread).map(|slot| Ending {
        root: slot.pid,
        gone: slot.reaped || slot.exit.is_some(),
        spare: if with_root {
            tree::Known::new()
        } else {
            slot.spared.clone()
        },
        // A stop sweeps what was seen; so does each command on a program that ended and left
        // a process that a sweep could not end.
        seen: if with_root || slot.left_behind {
            slot.seen.clone()
        } else {
            tree::Known::new()
        },
        left_behind: slot.left_behind,
    });
    match found {
        None => refused(Refusal::UnknownThread),
        Some(Ending {
            gone: true,
            left_behind: false,
            ..
        }) => Outcome::Stopped {
            ended: 0,
            all_ended: true,
        },
        Some(Ending {
            gone: true, seen, ..
        }) => {
            // The program ended and its sweep did not prove that all ended: sweep again.
            let swept = tree::sweep(&seen).await;
            if let Some(slot) = locked(shared).threads.get_mut(thread) {
                program::settle_sweep(slot, swept.all);
            }
            Outcome::Stopped {
                ended: swept.count,
                all_ended: swept.all,
            }
        }
        Some(ending) => {
            let ended = tree::end(ending.root, with_root, &ending.spare, GRACE).await;
            // A process that was below the program and left its tree (its parent ended) is
            // found by its start time (THR-049).
            let swept = tree::sweep(&ending.seen).await;
            Outcome::Stopped {
                ended: ended.count.saturating_add(swept.count),
                all_ended: ended.all && swept.all,
            }
        }
    }
}

fn forget(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey) -> Outcome {
    let Ok(mut state) = shared.lock() else {
        return refused(Refusal::UnknownThread);
    };
    match state.threads.get(thread).map(|slot| slot.exit.is_some()) {
        None => refused(Refusal::UnknownThread),
        Some(false) => refused(Refusal::Busy),
        Some(true) => {
            state.threads.remove(thread);
            Outcome::Ok
        }
    }
}

/// Ends each program the holder holds, with the processes below it (THR-049), at the end of the
/// holder.
pub async fn end_all(shared: &Arc<Mutex<Shared>>) {
    let running: Vec<ThreadKey> = locked(shared)
        .threads
        .iter()
        .filter(|(_, slot)| slot.exit.is_none())
        .map(|(key, _)| key.clone())
        .collect();
    for thread in running {
        close_input(shared, &thread);
        end(shared, &thread, true).await;
    }
    locked(shared).write_record();
}
