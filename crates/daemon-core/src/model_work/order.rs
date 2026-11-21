//! The check of a model item before `preparing` (Section 5, CAG-004, CAG-007): one pure
//! function. It refuses for the first cause that is true; the state of the machine is a
//! parameter.

mod parameters;

use serde_json::{Map, Value};
use uuid::Uuid;

pub use parameters::{Parameters, PullFile, PullSource};

use super::cause::Cause;
use super::machine::MachineSettings;
use super::names::{Actor, Word};
use super::settings::SettingLimits;
use crate::wire::WorkType;

type Object = Map<String, Value>;

/// The members of a model item (K10 Section 4 without `repository`, with `actor`).
const MEMBERS: [&str; 10] = [
    "id",
    "type",
    "kind",
    "attempt",
    "lease",
    "limits",
    "project",
    "secrets",
    "parameters",
    "actor",
];

/// What the machine is and permits at the moment of the check.
#[derive(Clone, Copy, Debug)]
pub struct ModelMachine<'a> {
    /// The switch at the machine (USE-004).
    pub switch_on: bool,
    /// The device policy permits work now (USE-009).
    pub policy_permits: bool,
    pub settings: &'a MachineSettings,
    pub limits: SettingLimits<'a>,
}

/// A model item that passed each check.
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub id: Uuid,
    pub attempt: u32,
    pub work_type: WorkType,
    pub actor: Actor,
    pub parameters: Parameters,
}

/// The answer of the check.
#[derive(Clone, Debug, PartialEq)]
pub enum Checked {
    Start(Box<Order>),
    Refuse {
        id: Uuid,
        attempt: u32,
        cause: Cause,
    },
    /// No item, or no id and attempt that a report could name.
    Drop,
}

fn secrets_hold(item: &Object, work_type: WorkType, parameters: &Parameters) -> Result<(), Cause> {
    let list = item
        .get("secrets")
        .and_then(Value::as_array)
        .ok_or(Cause::MemberUnknown)?;
    let gated = matches!(parameters, Parameters::Pull { gated: true, .. });
    let names: Vec<&str> = list.iter().filter_map(Value::as_str).collect();
    let wanted: &[&str] = if gated { &["hf_token"] } else { &[] };
    if names.len() != list.len() || (work_type != WorkType::ModelPull && !names.is_empty()) {
        return Err(Cause::SecretUnknown);
    }
    (names == wanted).then_some(()).ok_or(Cause::SecretUnknown)
}

/// The types that only reduce the work of the machine: never refused for the switch or the
/// device policy (Section 3).
const REDUCING: [WorkType; 5] = [
    WorkType::ServerStop,
    WorkType::ModelUnload,
    WorkType::ModelDrain,
    WorkType::CopyDelete,
    WorkType::ModelReport,
];

fn cause(item: &Object, machine: &ModelMachine<'_>) -> Result<Order, Cause> {
    let work_type = item
        .get("type")
        .and_then(Value::as_str)
        .and_then(WorkType::parse)
        .filter(|kind| kind.is_model())
        .ok_or(Cause::TypeUnknown)?;
    if item.get("kind").and_then(Value::as_str) != Some("model") {
        return Err(Cause::KindNotPermitted);
    }
    let members_hold = item.len() == MEMBERS.len()
        && MEMBERS.iter().all(|name| item.contains_key(*name))
        && item.get("project").is_some_and(Value::is_null);
    let actor = item
        .get("actor")
        .and_then(Value::as_str)
        .and_then(Actor::parse)
        .filter(|_| members_hold)
        .ok_or(Cause::MemberUnknown)?;
    let object = item
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or(Cause::MemberUnknown)?;
    let parameters = parameters::read(work_type, object, machine)?;
    secrets_hold(item, work_type, &parameters)?;
    if !REDUCING.contains(&work_type) {
        if !machine.switch_on {
            return Err(Cause::SwitchOff);
        }
        if !machine.policy_permits {
            return Err(Cause::Policy);
        }
    }
    if !machine.settings.actor_permits(actor, work_type) {
        return Err(Cause::ActorNotPermitted);
    }
    Ok(Order {
        id: Uuid::nil(),
        attempt: 0,
        work_type,
        actor,
        parameters,
    })
}

/// Checks the answer to a claim. The daemon does nothing for an answer that is not
/// [`Checked::Start`].
#[must_use]
pub fn check(answer: &Value, machine: &ModelMachine<'_>) -> Checked {
    let Some(item) = answer.as_object() else {
        return Checked::Drop;
    };
    let id = item
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok());
    let attempt = item
        .get("attempt")
        .and_then(Value::as_u64)
        .and_then(|attempt| u32::try_from(attempt).ok())
        .filter(|attempt| *attempt >= 1);
    let (Some(id), Some(attempt)) = (id, attempt) else {
        return Checked::Drop;
    };
    match cause(item, machine) {
        Ok(order) => Checked::Start(Box::new(Order {
            id,
            attempt,
            ..order
        })),
        Err(cause) => Checked::Refuse { id, attempt, cause },
    }
}
