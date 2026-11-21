//! What the holder does for each command of the daemon. A command is done one time: the same
//! ID again gets the first answer and does nothing (H10).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncWriteExt as _;

use super::holder::Shared;
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
        shared
            .lock()
            .ok()
            .and_then(|state| state.done.get(&command.id).cloned())
    };
    let outcome = if let Some(outcome) = known {
        outcome
    } else {
        let outcome = act(shared, command.action).await;
        if let Ok(mut state) = shared.lock() {
            state.remember(command.id, outcome.clone());
        }
        outcome
    };
    if let Ok(state) = shared.lock() {
        state.send(&Event::Done {
            id: command.id,
            outcome: outcome.clone(),
        });
    }
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
        Action::Write { thread, line } => write(shared, &thread, line).await,
        Action::CloseInput { thread } => match input(shared, &thread) {
            Some(input) => {
                input.lock().await.take();
                Outcome::Ok
            }
            None => refused(Refusal::UnknownThread),
        },
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
            if let Some(input) = input(shared, &thread) {
                input.lock().await.take();
            }
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
    let input = Arc::new(tokio::sync::Mutex::new(child.stdin.take()));
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

fn input(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey) -> Option<program::Input> {
    let state = shared.lock().ok()?;
    state
        .threads
        .get(thread)
        .map(|slot| Arc::clone(&slot.input))
}

async fn write(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey, mut line: String) -> Outcome {
    let Some(input) = input(shared, thread) else {
        return refused(Refusal::UnknownThread);
    };
    let mut input = input.lock().await;
    let Some(pipe) = input.as_mut() else {
        return refused(Refusal::InputClosed);
    };
    if !line.ends_with('\n') {
        line.push('\n');
    }
    match pipe.write_all(line.as_bytes()).await {
        Ok(()) => Outcome::Ok,
        Err(_) => refused(Refusal::InputClosed),
    }
}

async fn end(shared: &Arc<Mutex<Shared>>, thread: &ThreadKey, with_root: bool) -> Outcome {
    let pid = shared.lock().ok().and_then(|state| {
        state
            .threads
            .get(thread)
            .map(|slot| (slot.pid, slot.reaped || slot.exit.is_some()))
    });
    match pid {
        None => refused(Refusal::UnknownThread),
        Some((_, true)) => Outcome::Stopped {
            ended: 0,
            all_ended: true,
        },
        Some((pid, false)) => {
            let ended = tree::end(pid, with_root, GRACE).await;
            Outcome::Stopped {
                ended: ended.count,
                all_ended: ended.all,
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
    let running: Vec<ThreadKey> = shared.lock().map_or_else(
        |_| Vec::new(),
        |state| {
            state
                .threads
                .iter()
                .filter(|(_, slot)| slot.exit.is_none())
                .map(|(key, _)| key.clone())
                .collect()
        },
    );
    for thread in running {
        if let Some(input) = input(shared, &thread) {
            input.lock().await.take();
        }
        end(shared, &thread, true).await;
    }
    if let Ok(state) = shared.lock() {
        state.write_record();
    }
}
