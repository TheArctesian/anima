//! Reader of `df -P` (MET-002).

use super::{Disk, label, lines};

/// `df -P` output: one [`Disk`] per line of `Filesystem Blocks Used Available Capacity
/// Mounted-on`. `block_bytes` is the size of a block: 1 for `df -P -B1` (Linux), 1024 for
/// `df -P -k` (macOS). The mount point is the rest of the line, so it may hold spaces. The
/// header and any line that does not read are skipped.
#[must_use]
pub fn disks(output: &[u8], block_bytes: u64) -> Vec<Disk> {
    lines(output)
        .filter_map(|line| {
            let ([filesystem, total, used, _available, capacity], mount) = columns(line)?;
            if !capacity.ends_with('%') {
                return None;
            }
            Some(Disk {
                filesystem: filesystem.to_owned(),
                mount: label(mount)?.to_owned(),
                total_bytes: total.parse::<u64>().ok()?.checked_mul(block_bytes)?,
                used_bytes: used.parse::<u64>().ok()?.checked_mul(block_bytes)?,
            })
        })
        .collect()
}

/// The first five words of `line` and the rest of it, trimmed.
fn columns(line: &str) -> Option<([&str; 5], &str)> {
    let mut rest = line;
    let mut words = [""; 5];
    for word in &mut words {
        let (head, tail) = rest.trim_start().split_once(char::is_whitespace)?;
        *word = head;
        rest = tail;
    }
    Some((words, rest.trim()))
}
