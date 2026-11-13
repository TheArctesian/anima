//! A collection's GPU reads over its link: each within its own limit and all before a time the
//! collection sets, so a slow, missing or refused GPU tool never ends the collection (MET-006,
//! D-11, D-30). I/O edge; nothing here logs, and each output is read and dropped before the
//! next read runs.

use tokio::time::{Instant, timeout_at};

use super::answers::{Answer, answer};
use crate::fleet::{
    metrics::{GpuVendor, commands::GpuCommand},
    ssh::{LinkError, SshLink},
};

/// The answer of each of `commands`, in order, run on `link` until `until`. A read that times
/// out or is refused stays [`Answer::Unknown`] and the next one runs; after a lost link, and
/// past `until`, the reads not done stay unknown.
pub(in crate::fleet::metrics::collector) async fn ask(
    link: &mut SshLink,
    commands: &[GpuCommand],
    until: Instant,
) -> Vec<(GpuVendor, Answer)> {
    let mut answers: Vec<(GpuVendor, Answer)> = commands
        .iter()
        .map(|command| (command.vendor, Answer::Unknown))
        .collect();
    let asking = async {
        for (command, (_, slot)) in commands.iter().zip(answers.iter_mut()) {
            match link.run(&command.fixed, command.limit).await {
                Ok(output) => *slot = answer(command.vendor, &output),
                Err(LinkError::Lost) => break,
                // A time-out or a refusal leaves this read unknown; the link still runs the next.
                Err(_) => {}
            }
        }
    };
    // Past `until` the reads still to run stay unknown; the collection keeps the rest.
    let _in_time = timeout_at(until, asking).await;
    answers
}
