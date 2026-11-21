//! The stop of a program and of the processes below it (THR-049): the walk of the tree, the
//! sweep of what was seen below the program, and the processes that carry the mark of the
//! thread.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::commands::refused;
use super::holder::{Shared, locked};
use super::program;
use super::tree;
use super::wire::{Outcome, Refusal, ThreadKey};

/// The time the program of a thread has after `SIGTERM` before `SIGKILL`.
const GRACE: Duration = Duration::from_secs(3);

/// What the holder knows of a program when it ends its processes.
struct Ending {
    root: u32,
    gone: bool,
    spare: tree::Known,
    /// The processes seen below the program that may have left its tree since (a stop only).
    seen: tree::Known,
    left_behind: bool,
}

/// Ends the processes of the program of `thread`, and the program too when `with_root`.
pub(super) async fn end(
    shared: &Arc<Mutex<Shared>>,
    thread: &ThreadKey,
    with_root: bool,
) -> Outcome {
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
        }) => {
            let marked = super::marker::end_marked(thread).await;
            Outcome::Stopped {
                ended: marked.count,
                all_ended: marked.all,
            }
        }
        Some(Ending {
            gone: true, seen, ..
        }) => {
            // The program ended and its sweep did not prove that all ended: sweep again.
            let swept = tree::sweep(&seen).await;
            let marked = super::marker::end_marked(thread).await;
            let all = swept.all && marked.all;
            if let Some(slot) = locked(shared).threads.get_mut(thread) {
                program::settle_sweep(slot, all);
            }
            Outcome::Stopped {
                ended: swept.count.saturating_add(marked.count),
                all_ended: all,
            }
        }
        Some(ending) => {
            let ended = tree::end(ending.root, with_root, &ending.spare, GRACE).await;
            // A process that was below the program and left its tree (its parent ended) is
            // found by its start time (THR-049).
            let swept = tree::sweep(&ending.seen).await;
            // A process that left the tree between two looks is found by its mark.
            let marked = super::marker::end_marked(thread).await;
            Outcome::Stopped {
                ended: ended
                    .count
                    .saturating_add(swept.count)
                    .saturating_add(marked.count),
                all_ended: ended.all && swept.all && marked.all,
            }
        }
    }
}

/// Ends each program the holder holds, with the processes below it (THR-049), at the end of the
/// holder.
pub async fn end_all(shared: &Arc<Mutex<Shared>>) {
    let running: Vec<ThreadKey> = locked(shared)
        .threads
        .iter()
        // A program that ended but left a process its sweep could not end is swept again.
        .filter(|(_, slot)| slot.exit.is_none() || slot.left_behind)
        .map(|(key, _)| key.clone())
        .collect();
    for thread in running {
        super::commands::close_input(shared, &thread);
        end(shared, &thread, true).await;
    }
    locked(shared).write_record();
}
