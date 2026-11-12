//! Reader of `lsblk -J -b`: the drives of a Linux device, their kind and the mounts on each
//! (MET-002, MET-022).
//!
//! The output is the device's text: a document that does not parse gives `None`, and each
//! name and mount point must pass the series label rule. The tree is walked at most
//! [`DEPTH`] levels down, so a hostile document cannot make the walk deep.

use serde::Deserialize;

use super::label;
use crate::points::{MAX_LABELS, is_operator_mount};

/// The arguments of the `lsblk` command (util-linux 2.37 and later, which has `MOUNTPOINTS`).
pub const LSBLK_ARGS: [&str; 4] = ["-J", "-b", "-o", "NAME,TYPE,SIZE,ROTA,TRAN,MOUNTPOINTS"];

/// The arguments for an older util-linux, which refuses `MOUNTPOINTS`: one mount per device.
pub const LSBLK_OLD_ARGS: [&str; 4] = ["-J", "-b", "-o", "NAME,TYPE,SIZE,ROTA,TRAN,MOUNTPOINT"];

/// How many levels below a drive its mounts are looked for: partition, crypt, LVM and RAID
/// stack a few levels at most.
const DEPTH: usize = 8;

/// Kernel devices that are not drives: loop images, RAM disks and compressed swap in memory.
const NOT_DRIVES: [&str; 3] = ["loop", "ram", "zram"];

/// What a drive is made of, read from its transport and rotation, never entered (MET-022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DriveKind {
    Nvme,
    Ssd,
    Hdd,
    /// Neither the transport nor the rotation said.
    Disk,
}

impl DriveKind {
    /// The text kept in `metrics.drives.kind` and sent in a batch's `drives`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nvme => "nvme",
            Self::Ssd => "ssd",
            Self::Hdd => "hdd",
            Self::Disk => "disk",
        }
    }

    /// The kind stored as `text`; `None` for text no kind has.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Nvme, Self::Ssd, Self::Hdd, Self::Disk]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }

    /// `NVMe` for transport `nvme`, else SSD or HDD by rotation, else Disk.
    fn of(transport: Option<&str>, rotational: Option<bool>) -> Self {
        match (transport, rotational) {
            (Some("nvme"), _) => Self::Nvme,
            (_, Some(false)) => Self::Ssd,
            (_, Some(true)) => Self::Hdd,
            (_, None) => Self::Disk,
        }
    }
}

/// One drive of a device and the operator's mounts on it (MET-022).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drive {
    pub name: String,
    pub kind: DriveKind,
    /// At most `i64::MAX`, so the hub can store it.
    pub size_bytes: u64,
    /// The mount points on the drive and on what sits on it, sorted bytewise, each once, at
    /// most [`MAX_LABELS`]; empty when it is not mounted.
    pub mounts: Vec<String>,
}

#[derive(Deserialize)]
struct Listing {
    blockdevices: Vec<Node>,
}

/// One device of the tree. Every member may be missing or `null`; util-linux before 2.33
/// printed numbers and flags as strings, later versions as numbers and booleans.
#[derive(Deserialize)]
struct Node {
    name: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    size: Option<Loose>,
    rota: Option<Loose>,
    tran: Option<String>,
    mountpoints: Option<Vec<Option<String>>>,
    mountpoint: Option<String>,
    #[serde(default)]
    children: Vec<Node>,
}

/// A value as any util-linux version prints it.
#[derive(Deserialize)]
#[serde(untagged)]
enum Loose {
    Number(u64),
    Flag(bool),
    Text(String),
}

impl Loose {
    fn number(&self) -> Option<u64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Text(text) => text.trim().parse().ok(),
            Self::Flag(_) => None,
        }
    }

    fn flag(&self) -> Option<bool> {
        match self {
            Self::Flag(flag) => Some(*flag),
            Self::Number(0) => Some(false),
            Self::Number(1) => Some(true),
            Self::Text(text) => match text.trim() {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            },
            Self::Number(_) => None,
        }
    }
}

/// The drives in `lsblk -J -b` output, by name (bytewise), each once, at most
/// [`MAX_LABELS`]. `None` when the output is not such a document.
#[must_use]
pub fn drives(output: &[u8]) -> Option<Vec<Drive>> {
    let listing: Listing = serde_json::from_slice(output).ok()?;
    let mut drives: Vec<Drive> = listing.blockdevices.iter().filter_map(drive).collect();
    drives.sort_by(|a, b| a.name.cmp(&b.name));
    drives.dedup_by(|later, earlier| later.name == earlier.name);
    drives.truncate(MAX_LABELS);
    Some(drives)
}

/// The drive `node` is: a top-level `disk` with a usable name and a size above zero.
fn drive(node: &Node) -> Option<Drive> {
    let name = label(node.name.as_deref()?)?;
    let is_drive = node.kind.as_deref() == Some("disk")
        && !NOT_DRIVES.iter().any(|prefix| name.starts_with(prefix));
    let size_bytes = node.size.as_ref()?.number()?;
    if !is_drive || size_bytes == 0 || i64::try_from(size_bytes).is_err() {
        return None;
    }
    let mut mounts = Vec::new();
    collect_mounts(node, 0, &mut mounts);
    mounts.sort();
    mounts.dedup();
    mounts.truncate(MAX_LABELS);
    Some(Drive {
        name: name.to_owned(),
        kind: DriveKind::of(
            node.tran.as_deref(),
            node.rota.as_ref().and_then(Loose::flag),
        ),
        size_bytes,
        mounts,
    })
}

/// Adds the operator's mounts of `node` and of what sits on it, to [`DEPTH`] levels.
fn collect_mounts(node: &Node, depth: usize, mounts: &mut Vec<String>) {
    let listed = node.mountpoints.iter().flatten().flatten();
    for mount in listed.chain(&node.mountpoint) {
        if let Some(mount) = label(mount).filter(|mount| wanted(mount)) {
            mounts.push(mount.to_owned());
        }
    }
    if depth < DEPTH {
        for child in &node.children {
            collect_mounts(child, depth + 1, mounts);
        }
    }
}

/// A path the operator's files live under; swap's `[SWAP]` is not a path.
fn wanted(mount: &str) -> bool {
    mount.starts_with('/') && is_operator_mount(mount)
}
