//! The release that a machine gets next on its way to a version (ROL-041).

use super::model::{Release, Target};
use crate::fleet::builds::Version;

/// The version that a machine of `target` with `installed` gets next, to arrive at `aim`:
/// `aim` itself when its release can replace `installed`, or else the highest release below it
/// that can. `None` when the hub does not hold the release of `aim` for that target, when no
/// held release can replace `installed`, or when `installed` is not below `aim`.
#[must_use]
pub fn next_hop(
    installed: Version,
    target: Target,
    aim: Version,
    held: &[Release],
) -> Option<Version> {
    let of_target = || held.iter().filter(|release| release.target == target);
    if installed >= aim || !of_target().any(|release| release.version == aim) {
        return None;
    }
    of_target()
        .filter(|release| release.version > installed && release.version <= aim)
        .filter(|release| release.replaces_from <= installed)
        .map(|release| release.version)
        .max()
}
