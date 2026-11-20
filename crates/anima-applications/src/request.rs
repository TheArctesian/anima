//! One container item as the daemon gives it: the `Item` of contract K10 Section 4 with the
//! `parameters` of its type (Sections 6.4.6 to 6.4.11). The reader is strict: the program
//! does not trust the daemon and checks each member (Section 6.4.1).

mod parameters;

pub use parameters::{Action, AdoptedAction, Apply, Pruned, Removal, Work};

use crate::{
    Refusal,
    forms::is_uuid,
    strict::{self, Members, Value},
};

/// The most bytes of the text of one item.
pub const LARGEST_REQUEST: usize = 16 * 1024;

const UNKNOWN: Refusal = Refusal::MemberUnknown;

/// One container item that has the form of the contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The identifier of the item: a UUID of 36 lower-case characters.
    pub id: String,
    /// `limits.seconds`: the longest time of a command of this item.
    pub seconds: u64,
    pub work: Work,
}

/// The lease is `{ "until": <a time> }`. The program does not use the time; it checks only
/// that the text can hold no other thing than a time.
fn lease(value: Option<Value>) -> Option<()> {
    let mut members = Members::of(value?)?;
    let until = members.text("until")?;
    let sound = (1..=40).contains(&until.len())
        && until
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b"-:.TZ+".contains(&byte));
    (sound && members.is_empty()).then_some(())
}

/// `limits` has the five members of Section 4, each a whole number. Only `seconds` has a
/// meaning for the kind `host` (Section 7): it is above 0 and at most `most`.
fn seconds(value: Option<Value>, most: u64) -> Option<u64> {
    let mut members = Members::of(value?)?;
    let seconds = members.whole("seconds")?;
    for ignored in ["idleSeconds", "memoryBytes", "diskBytes", "orphanSeconds"] {
        members.whole(ignored)?;
    }
    (members.is_empty() && (1..=most).contains(&seconds)).then_some(seconds)
}

/// `secrets` names only the purposes that the type has (Section 8): `compose_file` for each
/// `application_apply`, and `environment_file` when the item has an environment file.
fn secrets(named: &[String], work: &Work) -> Result<(), Refusal> {
    let known = |purpose: &String| matches!(purpose.as_str(), "compose_file" | "environment_file");
    let of_an_apply = matches!(work, Work::Apply(_));
    if !named.iter().all(known) || (!of_an_apply && !named.is_empty()) {
        return Err(Refusal::SecretUnknown);
    }
    let Work::Apply(apply) = work else {
        return Ok(());
    };
    let count = |purpose: &str| named.iter().filter(|name| *name == purpose).count();
    let environment = usize::from(apply.environment_sha256.is_some());
    if count("compose_file") == 1 && count("environment_file") == environment {
        Ok(())
    } else {
        Err(UNKNOWN)
    }
}

/// Reads the text of one item.
///
/// # Errors
/// `type_unknown` for a type that is not a container type; `kind_not_permitted` for a kind
/// other than `host`; `secret_unknown` for a purpose that the type does not have;
/// `member_unknown` for text that is not strict JSON, a member that the contract does not
/// name, a member that is not there, and a value that breaks a rule of the contract.
pub fn read(text: &str) -> Result<Request, Refusal> {
    if text.len() > LARGEST_REQUEST {
        return Err(UNKNOWN);
    }
    let mut members = strict::read(text).and_then(Members::of).ok_or(UNKNOWN)?;
    let kind = members.text("type").ok_or(UNKNOWN)?;
    let parameters = members.take("parameters");
    if !parameters::is_container_type(&kind) {
        return Err(Refusal::TypeUnknown);
    }
    if members.text("kind").as_deref() != Some("host") {
        return Err(Refusal::KindNotPermitted);
    }
    let work = parameters::read(&kind, parameters.ok_or(UNKNOWN)?)?;
    let id = members.text("id").filter(|id| is_uuid(id)).ok_or(UNKNOWN)?;
    // The hub makes no second attempt of a container item (Section 7).
    if members.whole("attempt") != Some(1) {
        return Err(UNKNOWN);
    }
    lease(members.take("lease")).ok_or(UNKNOWN)?;
    let seconds = seconds(members.take("limits"), work.most_seconds()).ok_or(UNKNOWN)?;
    // An item of a container type has no `repository`, and its `project` is null (Section 4).
    if members.take("project") != Some(Value::Null) {
        return Err(UNKNOWN);
    }
    secrets(&members.texts("secrets").ok_or(UNKNOWN)?, &work)?;
    if !members.is_empty() {
        return Err(UNKNOWN);
    }
    Ok(Request { id, seconds, work })
}
