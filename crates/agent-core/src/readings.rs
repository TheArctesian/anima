//! Readers of device command output (MET-002). Each takes the bytes a command printed and gives
//! typed readings; none panics and none fails on one bad line.
//!
//! The commands are fixed text; the forms read here are the ones the collector runs:
//!
//! - Linux: `cat /proc/stat`, `cat /proc/meminfo`, `cat /proc/net/dev`, `cat /proc/uptime`,
//!   `cat /proc/loadavg`, `df -P -B1` (block size 1, see [`df::disks`]).
//! - macOS: `sysctl -n hw.memsize hw.pagesize vm.loadavg kern.boottime`, `vm_stat`,
//!   `df -P -k` (block size 1024), `netstat -ib -n`, `top -l 2 -n 0 -s 1`.
//!
//! A last line without its newline is dropped: the link cuts output at a limit, and half a
//! line can hold a wrong number. A line that is not UTF-8 is skipped.

#[cfg(any(test, feature = "test-support"))]
pub mod captures;
pub mod df;
pub mod linux;
pub mod macos;
mod reading;
#[cfg(test)]
mod tests;

pub use reading::{Cpu, Disk, Interface, Load, Memory, Reading};

/// The longest label (mount point or interface name) kept, in bytes.
const LABEL_LIMIT: usize = 128;

/// The complete lines of `output`, each `None` when it is not UTF-8 (so positions hold).
fn positional_lines(output: &[u8]) -> impl Iterator<Item = Option<&str>> {
    let complete = output
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |end| end + 1);
    output
        .get(..complete)
        .unwrap_or_default()
        .split_inclusive(|byte| *byte == b'\n')
        .map(|line| std::str::from_utf8(line).ok())
}

/// The complete lines of `output` that are UTF-8.
fn lines(output: &[u8]) -> impl Iterator<Item = &str> {
    positional_lines(output).flatten()
}

/// A finite number of at least zero; `NaN`, infinities and negative text give `None`.
fn number(text: &str) -> Option<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && !value.is_sign_negative())
}

/// `text` when it can name a series: 1 to 128 bytes, no control character, no replacement
/// character.
fn label(text: &str) -> Option<&str> {
    let usable = !text.is_empty()
        && text.len() <= LABEL_LIMIT
        && !text
            .chars()
            .any(|c| c.is_control() || c == char::REPLACEMENT_CHARACTER);
    usable.then_some(text)
}

/// Three load averages as the first three words of `words`.
fn load<'a>(mut words: impl Iterator<Item = &'a str>) -> Option<Load> {
    Some(Load {
        one: number(words.next()?)?,
        five: number(words.next()?)?,
        fifteen: number(words.next()?)?,
    })
}
