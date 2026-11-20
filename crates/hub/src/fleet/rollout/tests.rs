//! Tests of the pure rules of a rollout.

mod instruction;
mod membership;
mod progress;
mod state;
mod steps;

use chrono::{DateTime, TimeZone as _, Utc};

use super::{Cause, Machine, Outcome, Reach, Release, Said, Target, Try};
use crate::{fleet::builds::Version, ids::DeviceId};

const OLD: &str = "0.2.3";
const NEW: &str = "0.3.0";

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000 + seconds, 0).unwrap()
}

fn version(text: &str) -> Version {
    text.parse().unwrap()
}

fn device(number: u8) -> DeviceId {
    format!("0190a000-0000-7000-8001-0000000000{number:02}")
        .parse()
        .unwrap()
}

/// A connected Linux machine with `OLD`, with the update function, that never tried an update.
fn machine(number: u8) -> Machine {
    Machine {
        device: device(number),
        installed: Some(version(OLD)),
        reach: Reach::Connected,
        with_hub: false,
        held: false,
        said: Some(Said {
            target: Some(Target::LinuxX8664),
            switch_on: true,
            previous: None,
            busy: false,
            last: None,
        }),
    }
}

fn with_said(mut machine: Machine, change: impl FnOnce(&mut Said)) -> Machine {
    change(machine.said.as_mut().unwrap());
    machine
}

fn tried(machine: Machine, text: &str, outcome: Outcome, tries: u8) -> Machine {
    with_said(machine, |said| {
        said.last = Some(Try {
            version: version(text),
            outcome,
            tries,
        });
    })
}

/// The machine after a good update to `text`: it has the version and wrote `installed`.
fn updated(mut machine: Machine, text: &str) -> Machine {
    machine.installed = Some(version(text));
    tried(machine, text, Outcome::Installed, 1)
}

fn rolled_back(machine: Machine, text: &str, tries: u8) -> Machine {
    tried(machine, text, Outcome::RolledBack(Cause::NoCheck), tries)
}

/// A Linux release that can replace each version from `from`.
fn release(text: &str, from: &str) -> Release {
    Release {
        version: version(text),
        target: Target::LinuxX8664,
        replaces_from: version(from),
    }
}
