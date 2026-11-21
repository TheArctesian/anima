//! The stop of a program and of the processes below it (THR-049): the walk of the tree, the
//! sweep of what was seen below the program, and the processes that carry the mark of the
//! thread.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::commands::refused;
use super::holder::{Shared, Slot, locked};
use super::marker::Keep;
use super::program;
use super::tree;
use super::wire::{Outcome, Refusal, ThreadKey};

/// The time the program of a thread has after `SIGTERM` before `SIGKILL`.
const GRACE: Duration = Duration::from_secs(3);

/// What the holder knows of a program when it ends its processes.
struct Ending {
    root: u32,
    /// The start of the program, when only its children end.
    since: Option<String>,
    gone: bool,
    spare: tree::Known,
    /// The processes seen below the program that may have left its tree since (a stop only).
    seen: tree::Known,
    left_behind: bool,
}

impl Ending {
    /// What the holder knows of the program in `slot` for an end of its children, or of it too
    /// when `with_root`.
    fn of(slot: &Slot, with_root: bool) -> Self {
        Self {
            root: slot.pid,
            // An unknown start reads as no time, so that the sweep after a turn ends nothing.
            since: (!with_root).then(|| slot.started.clone().unwrap_or_default()),
            gone: slot.reaped || slot.exit.is_some(),
            spare: if with_root {
                tree::Known::new()
            } else {
                slot.spared.clone()
            },
            // A stop sweeps what was seen; so does each command on a program that ended and
            // left a process that a sweep could not end.
            seen: if with_root || slot.left_behind {
                slot.seen.clone()
            } else {
                tree::Known::new()
            },
            left_behind: slot.left_behind,
        }
    }

    /// What the sweep by mark keeps; `program` when only the children end.
    fn keep(&self, program: Option<u32>) -> Keep<'_> {
        Keep {
            program,
            since: self.since.as_deref(),
            spare: &self.spare,
        }
    }
}

/// Ends the processes of the program of `thread`, and the program too when `with_root`.
pub(super) async fn end(
    shared: &Arc<Mutex<Shared>>,
    thread: &ThreadKey,
    with_root: bool,
) -> Outcome {
    let found = locked(shared)
        .threads
        .get(thread)
        .map(|slot| Ending::of(slot, with_root));
    match found {
        None => refused(Refusal::UnknownThread),
        Some(ending) if ending.gone => end_after_exit(shared, thread, &ending).await,
        Some(ending) => {
            let ended = tree::end(ending.root, with_root, &ending.spare, GRACE).await;
            // A process that was below the program and left its tree (its parent ended) is
            // found by its start time (THR-049).
            let swept = tree::sweep(&ending.seen).await;
            // A process that left the tree between two looks is found by its mark. The ending
            // of the children keeps the program, which carries the mark too.
            let keep = ending.keep((!with_root).then_some(ending.root));
            let marked = super::marker::end_marked(thread, &keep).await;
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

/// The end of what a program that ended left: when its sweep did not prove that all ended,
/// sweep again; then the processes that carry its mark.
async fn end_after_exit(
    shared: &Arc<Mutex<Shared>>,
    thread: &ThreadKey,
    ending: &Ending,
) -> Outcome {
    let swept = if ending.left_behind {
        tree::sweep(&ending.seen).await
    } else {
        tree::Ended {
            count: 0,
            all: true,
        }
    };
    let marked = super::marker::end_marked(thread, &ending.keep(None)).await;
    let all = swept.all && marked.all;
    if ending.left_behind
        && let Some(slot) = locked(shared).threads.get_mut(thread)
    {
        program::settle_sweep(slot, all);
    }
    Outcome::Stopped {
        ended: swept.count.saturating_add(marked.count),
        all_ended: all,
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
