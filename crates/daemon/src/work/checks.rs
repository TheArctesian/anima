//! What the daemon does with an item before it starts it (K10 Section 5): it checks the item
//! and refuses it for the first cause of the fixed list that is true. The hub names no command,
//! and the machine decides: the switch, the kinds, the repositories and the isolation are the
//! machine's (K10 Section 3). One pure function; the state of the machine is a parameter.

use daemon_core::wire::{WorkJob, WorkKind};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::{
    permitted::Repositories,
    refusal::Cause,
    shape,
    types::{self, Version},
};

type Object = Map<String, Value>;

/// What the machine is and permits at the moment of the check.
#[derive(Clone, Copy, Debug)]
pub struct Machine<'a> {
    /// The switch at the machine (USE-004). It can go off after the claim.
    pub switch_on: bool,
    /// The device policy permits work now (USE-009).
    pub policy_permits: bool,
    /// The kinds that the owner permits at this machine (PRO-024).
    pub kinds: &'a [WorkKind],
    /// The kinds whose isolation the machine can make now (task W2).
    pub isolation: &'a [WorkKind],
    /// The last clean-up did not delete each thing (K10 Section 9).
    pub unclean: bool,
    pub repositories: &'a Repositories,
    /// The lowest version of the runner program that this daemon accepts (SF-014).
    pub lowest_runner: Version,
}

/// What the daemon does with an answer to a claim.
#[derive(Clone, Debug, PartialEq)]
pub enum Checked {
    /// Each check passed.
    Start(Box<WorkJob>),
    /// The daemon reports `refused` with the cause.
    Refuse {
        id: Uuid,
        attempt: u32,
        cause: Cause,
    },
    /// The answer names no item and no attempt: there is nothing that a report could name.
    Drop,
}

fn kind_of(item: &Object) -> Option<WorkKind> {
    match shape::text(item, "kind")? {
        "container" => Some(WorkKind::Container),
        "native" => Some(WorkKind::Native),
        _ => None,
    }
}

/// The checks of the item that need its type: the object of the type, the secrets and the
/// version of the runner program.
fn typed(item: &Object, work_type: &str, lowest: Version) -> Result<(), Cause> {
    let secrets = shape::purposes(item).ok_or(Cause::MemberUnknown)?;
    let parameters = item
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or(Cause::MemberUnknown)?;
    if !secrets
        .iter()
        .all(|purpose| types::purposes_of(work_type).contains(purpose))
    {
        return Err(Cause::SecretUnknown);
    }
    if work_type == "build" {
        return types::build_holds(parameters, &secrets)
            .then_some(())
            .ok_or(Cause::MemberUnknown);
    }
    let version = types::runner_version(parameters).ok_or(Cause::MemberUnknown)?;
    (version >= lowest).then_some(()).ok_or(Cause::RunnerTooOld)
}

/// The first cause of K10 Section 5 that is true for the item, in the order of its table but
/// `runner_too_old`, which is read with the object of the type.
fn cause(item: &Object, machine: &Machine<'_>) -> Result<WorkJob, Cause> {
    let work_type = shape::text(item, "type")
        .filter(|name| ["build", "runner"].contains(name))
        .ok_or(Cause::TypeUnknown)?;
    let kind = kind_of(item)
        .filter(|kind| machine.kinds.contains(kind))
        .ok_or(Cause::KindNotPermitted)?;
    if !shape::holds(item) {
        return Err(Cause::MemberUnknown);
    }
    typed(item, work_type, machine.lowest_runner)?;
    let job: WorkJob =
        serde_json::from_value(Value::Object(item.clone())).map_err(|_| Cause::MemberUnknown)?;
    let states = [
        (machine.switch_on, Cause::SwitchOff),
        (machine.policy_permits, Cause::Policy),
        (machine.isolation.contains(&kind), Cause::NoIsolation),
        (!machine.unclean, Cause::Unclean),
        (
            machine.repositories.permits(&job.repository),
            Cause::RepositoryNotPermitted,
        ),
    ];
    match states.into_iter().find(|(holds, _)| !holds) {
        Some((_, cause)) => Err(cause),
        None => Ok(job),
    }
}

/// Checks the answer to a claim. The daemon starts nothing for an answer that is not
/// [`Checked::Start`].
#[must_use]
pub fn check(answer: &Value, machine: &Machine<'_>) -> Checked {
    let Some(item) = answer.as_object() else {
        return Checked::Drop;
    };
    let id = shape::text(item, "id").and_then(|id| Uuid::parse_str(id).ok());
    let attempt = item
        .get("attempt")
        .and_then(Value::as_u64)
        .and_then(|attempt| u32::try_from(attempt).ok())
        .filter(|attempt| *attempt >= 1);
    let (Some(id), Some(attempt)) = (id, attempt) else {
        return Checked::Drop;
    };
    match cause(item, machine) {
        Ok(job) => Checked::Start(Box::new(job)),
        Err(cause) => Checked::Refuse { id, attempt, cause },
    }
}
