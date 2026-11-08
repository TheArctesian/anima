//! systemd service check: the command and the pure reading of its output (D-3, CHK-012).
//!
//! The hub runs `systemctl list-units --all --plain --no-legend --no-pager` through the SSH
//! link and looks for the unit among the first words of the rows. Nothing of the output is
//! kept: the result holds only fixed words (LOG-007).

use super::{
    outcome::{Failure, ProbeResult},
    ssh::readable,
};
use crate::ssh::{CommandOutput, FixedCommand};

/// The command, from static text only: the unit name is never part of it.
pub(in crate::checks) fn command() -> FixedCommand {
    FixedCommand::new("systemctl")
        .arg("list-units")
        .arg("--all")
        .arg("--plain")
        .arg("--no-legend")
        .arg("--no-pager")
}

/// The unit types systemd knows; a name that ends in none of them is a service.
const SUFFIXES: [&str; 11] = [
    ".service",
    ".socket",
    ".target",
    ".timer",
    ".mount",
    ".automount",
    ".swap",
    ".path",
    ".slice",
    ".scope",
    ".device",
];

/// The unit name as `systemctl` lists it: a bare name gets `.service`.
fn full_name(unit: &str) -> String {
    if SUFFIXES.iter().any(|suffix| unit.ends_with(suffix)) {
        unit.to_owned()
    } else {
        format!("{unit}.service")
    }
}

/// What the row's ACTIVE column says: Up only for `active`.
fn judge(load: &str, active: &str) -> ProbeResult {
    if load == "not-found" {
        return ProbeResult::failed(&Failure::UnitNotFound);
    }
    let class = match active {
        "active" => return ProbeResult::up_unmeasured(),
        "inactive" => "inactive",
        "failed" => "failed",
        "activating" => "activating",
        "deactivating" => "deactivating",
        "reloading" => "reloading",
        "maintenance" => "in maintenance",
        _ => "not active",
    };
    ProbeResult::failed(&Failure::UnitState(class))
}

/// The first three columns of a row (UNIT, LOAD, ACTIVE), after the bullet that marks a unit
/// in a bad state when the output is not plain.
fn columns(row: &str) -> Option<(&str, &str, &str)> {
    let mut words = row
        .split_whitespace()
        .skip_while(|word| matches!(*word, "\u{25cf}" | "*"));
    Some((words.next()?, words.next()?, words.next()?))
}

/// The result for `unit`: its row, or not found.
pub(in crate::checks) fn read(output: &CommandOutput, unit: &str) -> ProbeResult {
    let text = match readable(output) {
        Ok(text) => text,
        Err(failure) => return ProbeResult::failed(&failure),
    };
    let wanted = full_name(unit);
    text.lines()
        .filter_map(columns)
        .find(|(name, ..)| *name == wanted)
        .map_or_else(
            || ProbeResult::failed(&Failure::UnitNotFound),
            |(_, load, active)| judge(load, active),
        )
}
