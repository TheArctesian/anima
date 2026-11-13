//! What a maintenance window covers: `device:ID-or-name`, `check:ID` or `check:DEVICE/NAME`
//! and `tag:NAME`, as K1 `WindowTargetInput` values. Names become IDs here, as in every other
//! noun.

use serde_json::{Value, json};

use crate::checks::resolve::find as find_check;
use crate::devices::get::resolve as resolve_device;
use crate::failure::{Failure, Hint};
use crate::hub::Hub;

/// `device:ID`, `check:ID` or `tag:NAME` as a K1 `WindowTargetInput`, names still unresolved.
///
/// # Errors
/// [`Failure::Usage`] for a target that is not `TYPE:TARGET`.
pub fn parse(text: &str) -> Result<Value, Failure> {
    let usage = || Failure::Usage {
        message: format!(
            "{text} is not a target: use device:ID-or-name, check:ID, check:DEVICE/NAME or tag:NAME."
        ),
        hint: Some(Hint::run("anima maintenance create --help")),
    };
    let (kind, name) = text.split_once(':').ok_or_else(usage)?;
    let kind = match kind {
        "device" => "DEVICE",
        "check" => "CHECK",
        "tag" => "TAG",
        _ => return Err(usage()),
    };
    if name.is_empty() {
        return Err(usage());
    }
    Ok(json!({ "type": kind, "target": name }))
}

/// Replaces the device and check names in `input["targets"]` with their IDs.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device or check; [`Failure::Usage`] for a check name
/// without its device, or one that two devices share; the hub's refusal.
pub async fn resolve(hub: &Hub, input: &mut Value) -> Result<(), Failure> {
    let Some(targets) = input.get_mut("targets").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for target in targets {
        let name = target["target"].as_str().unwrap_or_default().to_owned();
        let id = match target["type"].as_str() {
            Some("DEVICE") => resolve_device(hub, &name).await?,
            Some("CHECK") => match name.split_once('/') {
                _ if uuid::Uuid::parse_str(&name).is_ok() => name,
                Some((device, check)) => find_check(hub, check, Some(device)).await?,
                None => {
                    return Err(Failure::Usage {
                        message: format!(
                            "{name} is not a check ID. Name its device too: check:DEVICE/NAME."
                        ),
                        hint: Some(Hint::run("anima maintenance create --help")),
                    });
                }
            },
            _ => continue,
        };
        target["target"] = json!(id);
    }
    Ok(())
}
