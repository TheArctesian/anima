//! The `extensions` tool set: which extensions of the hub are on, and turning one on or off
//! (EXT-011, EXT-012). Turning one on can turn others on with it; the result says which.

use std::collections::BTreeMap;

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::extensions::documents::{ExtensionList, ExtensionSet, extension_list, extension_set};
use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionTool {
    List,
    Set,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetInput {
    /// The extension's name in the hub, such as `projects` (see `anima_extensions_list`).
    extension: String,
    /// True to turn it on, false to turn it off.
    on: bool,
}

impl ExtensionTool {
    pub const ALL: [Self; 2] = [Self::List, Self::Set];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_extensions_list",
                Access::Read,
                Effect::Reads,
                "Lists the hub's extensions: key, name, whether it is built, fixed (part of the \
                 engine) and on, what it needs, what works with it and what needs it, and the \
                 menu entries it adds. Use it before turning one on or off.",
            )
            .returns(&[("extensions", "array", "Every extension.")]),
            Self::Set => Definition::new::<SetInput>(
                "anima_extension_set",
                Access::Login,
                Effect::Sets,
                "Turns an extension on or off. Turning one on also turns on the extensions it \
                 needs: the result lists them in `alsoChanged` and says so in `note`. The hub \
                 refuses to turn one off while another that is on needs it. Ask the operator \
                 first. Returns every extension.",
            )
            .returns(&[
                ("extensions", "array", "Every extension after the change."),
                (
                    "alsoChanged",
                    "array",
                    "Keys of other extensions turned on or off with this one.",
                ),
            ]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, an unknown extension or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => {
                let _: ListInput = parse(arguments)?;
                let listed = hub.run::<ExtensionList>(extension_list::Variables).await?;
                Ok(Value::Object(object([(
                    "extensions",
                    json(&listed.extensions),
                )])))
            }
            Self::Set => set(hub, arguments).await,
        }
    }
}

async fn set(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: SetInput = parse(arguments)?;
    let before = hub.run::<ExtensionList>(extension_list::Variables).await?;
    let was: BTreeMap<&str, bool> = before
        .extensions
        .iter()
        .map(|extension| (extension.key.as_str(), extension.on))
        .collect();
    if !was.contains_key(input.extension.as_str()) {
        return Err(ToolError::Refused(format!(
            "There is no extension with the key {}. Call anima_extensions_list for the keys.",
            input.extension
        )));
    }
    let variables = extension_set::Variables {
        key: input.extension.clone(),
        on: input.on,
    };
    let after = hub.run::<ExtensionSet>(variables).await?.set_extension;
    let now = after
        .iter()
        .map(|extension| (extension.key.as_str(), extension.on));
    let also = also_changed(&was, now, &input.extension);
    let note = if also.is_empty() {
        format!(
            "{} is now {}. No other extension changed.",
            input.extension,
            if input.on { "on" } else { "off" }
        )
    } else {
        format!(
            "{} is now {}. This also turned {} {}.",
            input.extension,
            if input.on { "on" } else { "off" },
            also.join(", "),
            if input.on { "on" } else { "off" }
        )
    };
    Ok(Value::Object(object([
        ("extensions", json(&after)),
        ("alsoChanged", json!(also)),
        ("note", json!(note)),
    ])))
}

/// The keys other than `key` whose state differs between `was` and `now`, sorted.
fn also_changed<'a>(
    was: &BTreeMap<&str, bool>,
    now: impl Iterator<Item = (&'a str, bool)>,
    key: &str,
) -> Vec<String> {
    let mut changed: Vec<String> = now
        .filter(|(each, on)| *each != key && was.get(each).is_some_and(|before| before != on))
        .map(|(each, _)| each.to_owned())
        .collect();
    changed.sort_unstable();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_011_only_other_extensions_whose_state_changed_are_named() {
        let was = BTreeMap::from([("projects", false), ("github", false), ("vault", true)]);
        let now = [("projects", true), ("github", true), ("vault", true)];
        assert_eq!(also_changed(&was, now.into_iter(), "projects"), ["github"]);
    }
}
