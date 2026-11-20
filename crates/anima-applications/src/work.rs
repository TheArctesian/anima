//! The check of each request (contract K10 Sections 6.4.1 to 6.4.3 and 6.4.6 to 6.4.11): for
//! one item and the facts of the machine, either the fixed code of a refusal or the plan of
//! what the program would do. Nothing here reads the engine, writes a file or starts a
//! program.
//!
//! A refusal of [`checked`] is found before the state `preparing`: the item is `refused`. A
//! refusal of [`apply::prepared`], [`apply::planned`] and [`back::prepared`] is found when the
//! program has the files: the item is `failed` with that cause (Section 5).

pub mod adopted;
pub mod apply;
pub mod back;
pub mod container;
pub mod prune;
pub mod removal;

use crate::{
    Inventory, Pair, PairDecision, PairRecord, Paths, Plan, Records, Refusal, Request, Settings,
    Work, pair,
    settings::{Machine, Managed},
};

/// What the program knows when it takes one item. The caller reads each fact; the checks
/// read none themselves.
#[derive(Clone, Copy, Debug)]
pub struct Facts<'a> {
    /// The settings of the machine, from the file of the administrator.
    pub settings: &'a Settings,
    /// The real paths that the program resolved at its start.
    pub paths: &'a Paths,
    /// What the engine has now.
    pub inventory: &'a Inventory,
    /// The applications that Anima installed, from the folders of the program.
    pub records: &'a Records,
    /// The record of the first pair of a hub, when the program has one.
    pub pair: Option<&'a PairRecord>,
    /// The memory, the processors and the free disk of the machine.
    pub machine: Machine,
    /// True while the program runs an item that changes Docker.
    pub busy: bool,
}

/// What a request that is not refused needs next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Accepted {
    /// The plan of the item.
    Ready(Plan),
    /// `application_apply`: the caller gets the two files and calls [`apply::prepared`].
    NeedsFiles,
    /// `application_back`: the caller reads the kept files and calls [`back::prepared`].
    NeedsKept,
}

/// The result of the check of one request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// The record of the pair that the program writes before each other thing, when the
    /// request changes it.
    pub record: Option<PairRecord>,
    /// The fixed code of the refusal, or what the item needs next.
    pub outcome: Result<Accepted, Refusal>,
}

/// The setting "Manage applications", when it is on.
///
/// # Errors
/// `applications_off` when it is off.
pub(crate) fn managed<'a>(facts: &Facts<'a>) -> Result<&'a Managed, Refusal> {
    facts
        .settings
        .managed
        .as_ref()
        .ok_or(Refusal::ApplicationsOff)
}

/// An item that changes Docker is refused while a different one runs.
///
/// # Errors
/// `busy`.
pub(crate) const fn free(facts: &Facts<'_>) -> Result<(), Refusal> {
    if facts.busy {
        Err(Refusal::Busy)
    } else {
        Ok(())
    }
}

fn of_the_type(work: &Work, facts: &Facts<'_>) -> Result<Accepted, Refusal> {
    match work {
        Work::Container { container, action } => {
            container::acted(container, *action, facts).map(Accepted::Ready)
        }
        Work::Follow { container, tail } => {
            container::followed(container, *tail, facts).map(Accepted::Ready)
        }
        Work::Apply(apply) => apply::accepted(apply, facts).map(|()| Accepted::NeedsFiles),
        Work::Back { application } => {
            back::accepted(application, facts).map(|()| Accepted::NeedsKept)
        }
        Work::Adopted {
            project,
            action,
            pull,
        } => adopted::applied(project, (*action, *pull), facts).map(Accepted::Ready),
        Work::Remove(removal) => removal::removed(removal, facts).map(Accepted::Ready),
        Work::Prune { what } => prune::pruned(what, facts).map(Accepted::Ready),
    }
}

/// Checks one request that `sent`, the pair of a hub, came with.
///
/// The order: "Manage applications" (`applications_off`); the pair of the hub
/// (`applications_off`); then the rules of the type.
#[must_use]
pub fn checked(request: &Request, sent: &Pair, facts: &Facts<'_>) -> Verdict {
    let Some(managed) = facts.settings.managed.as_ref() else {
        return Verdict {
            record: None,
            outcome: Err(Refusal::ApplicationsOff),
        };
    };
    let record = match pair::decided(facts.pair, managed.set_on_at, sent) {
        PairDecision::Recorded => None,
        PairDecision::First(record) => Some(record),
        PairDecision::Different(record) => {
            return Verdict {
                record,
                outcome: Err(Refusal::ApplicationsOff),
            };
        }
    };
    Verdict {
        record,
        outcome: of_the_type(&request.work, facts),
    }
}
