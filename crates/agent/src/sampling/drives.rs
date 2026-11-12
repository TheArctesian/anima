//! The drives a batch announces: the whole list after start and each time it changes, so an
//! unchanged list is not sent with every batch (MET-022, K8 Section 4).

use agent_core::readings::Drive;
use agent_core::wire::BatchDrive;

/// The drive list last handed to the pusher.
#[derive(Debug, Default)]
pub struct DriveWatch {
    announced: Option<Vec<Drive>>,
}

impl DriveWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `drives` member to send when `read` differs from the list announced last; `None`
    /// when it is the same, or when nothing could be read (the hub keeps what it has).
    pub fn changed(&mut self, read: Option<&[Drive]>) -> Option<Vec<BatchDrive>> {
        let read = read?;
        if self.announced.as_deref() == Some(read) {
            return None;
        }
        self.announced = Some(read.to_vec());
        Some(read.iter().map(BatchDrive::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use agent_core::readings::{captures::LSBLK_POP_OS, drives::drives};

    use super::*;

    #[test]
    fn met_022_the_drives_are_announced_first_then_only_when_they_change() {
        let mut watch = DriveWatch::new();
        let pop_os = drives(LSBLK_POP_OS).expect("a document");
        let first = watch.changed(Some(&pop_os)).expect("announced at start");
        assert_eq!(first.len(), 4);
        assert_eq!(watch.changed(Some(&pop_os)), None, "unchanged");
        assert_eq!(watch.changed(None), None, "unread is not empty");

        let mut mounted = pop_os.clone();
        mounted[2].mounts.push("/mnt/archive".to_owned());
        let again = watch.changed(Some(&mounted)).expect("a mount changed");
        assert_eq!(again[2].mounts, ["/mnt/archive"]);
        assert_eq!(
            watch.changed(Some(&[])),
            Some(Vec::new()),
            "every drive gone"
        );
    }
}
