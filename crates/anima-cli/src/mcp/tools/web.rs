//! The `web` tool set: a device's web interfaces (its admin pages). They are read through
//! `anima_device_get`. A URL with a user name or password in it is refused (MCP-004), and no
//! tool accepts a certificate (MCP-005).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::devices::get::resolve;
use crate::hub::Hub;
use crate::mcp::arguments::{confirmed, json, k1_input, object, parse, present};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::web_interfaces::documents::{
    WebCreate, WebDelete, WebList, WebUpdate, web_create, web_delete, web_list, web_update,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebTool {
    Create,
    Update,
    Delete,
}

/// Where the interface opens (K1 `OpenIn`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpenInInput {
    AnimaTab,
    SystemBrowser,
}

/// K1 `WebInterfaceInput`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceFields {
    /// The page's address, such as `https://10.10.0.5:8443`. No user name or password in it.
    url: String,
    /// A label; defaults to the URL's host.
    label: Option<String>,
    /// `ANIMA_TAB` (default) or `SYSTEM_BROWSER`.
    open_in: Option<OpenInInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInput {
    /// The device's id or exact name.
    device: String,
    #[serde(flatten)]
    fields: InterfaceFields,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    /// The device's id or exact name.
    device: String,
    /// The interface's id or exact label.
    interface: String,
    #[serde(flatten)]
    fields: InterfaceFields,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    /// The device's id or exact name.
    device: String,
    /// The interface's id or exact label.
    interface: String,
    /// The interface's exact label, as the operator confirmed it.
    confirm: String,
}

const INTERFACE: [(&str, &str, &str); 1] = [(
    "webInterface",
    "object",
    "id, deviceId, label, url, openIn, pinned certificate fingerprint.",
)];

impl WebTool {
    pub const ALL: [Self; 3] = [Self::Create, Self::Update, Self::Delete];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Create => Definition::new::<CreateInput>(
                "anima_web_interface_create",
                Access::Write,
                Effect::Adds,
                "Adds a web interface (an admin page the operator opens in Anima) to a device. \
                 The operator accepts its certificate in the client. Returns the interface.",
            )
            .returns(&INTERFACE),
            Self::Update => Definition::new::<UpdateInput>(
                "anima_web_interface_update",
                Access::Write,
                Effect::Sets,
                "Changes a web interface's URL, label or where it opens. Returns it.",
            )
            .returns(&INTERFACE),
            Self::Delete => Definition::new::<DeleteInput>(
                "anima_web_interface_delete",
                Access::Write,
                Effect::Removes,
                "Deletes a web interface of a device. Ask the operator first; confirm must be \
                 its exact label.",
            )
            .returns(&[("deleted", "string", "The deleted interface's id.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, a URL with credentials or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Create => create(hub, arguments).await,
            Self::Update => update(hub, arguments).await,
            Self::Delete => delete(hub, arguments).await,
        }
    }
}

async fn create(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CreateInput = parse(arguments)?;
    without_credentials(&input.fields.url)?;
    let device_id = resolve(hub, &input.device).await?;
    let variables = web_create::Variables {
        device_id,
        input: k1_input(present(&input.fields), "web interface")?,
    };
    let created = hub.run::<WebCreate>(variables).await?.create_web_interface;
    Ok(Value::Object(object([("webInterface", json(&created))])))
}

async fn update(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: UpdateInput = parse(arguments)?;
    without_credentials(&input.fields.url)?;
    let (id, _) = interface(hub, &input.device, &input.interface).await?;
    let variables = web_update::Variables {
        id,
        input: k1_input(present(&input.fields), "web interface")?,
    };
    let updated = hub.run::<WebUpdate>(variables).await?.update_web_interface;
    Ok(Value::Object(object([("webInterface", json(&updated))])))
}

async fn delete(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: DeleteInput = parse(arguments)?;
    let (id, label) = interface(hub, &input.device, &input.interface).await?;
    confirmed(&input.confirm, &label, "web interface")?;
    let deleted = hub
        .run::<WebDelete>(web_delete::Variables { id })
        .await?
        .delete_web_interface;
    Ok(Value::Object(object([("deleted", json!(deleted))])))
}

/// The id and label of the device's interface with this id or exact label.
async fn interface(hub: &Hub, device: &str, wanted: &str) -> Result<(String, String), ToolError> {
    let id = resolve(hub, device).await?;
    let interfaces = hub
        .run::<WebList>(web_list::Variables { id })
        .await?
        .device
        .map_or_else(Vec::new, |device| device.web_interfaces);
    let mut matches: Vec<(String, String)> = interfaces
        .into_iter()
        .filter(|candidate| candidate.id == wanted || candidate.label == wanted)
        .map(|found| (found.id, found.label))
        .collect();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(ToolError::Refused(
            "The device has no web interface with that id or label. Call anima_device_get to \
             list them."
                .to_owned(),
        )),
        _ => Err(ToolError::Refused(
            "More than one web interface has that label. Give its id (from anima_device_get)."
                .to_owned(),
        )),
    }
}

/// Refuses `https://user:password@host`: the user name and password would be stored in the
/// URL, outside the credential store (MCP-004).
fn without_credentials(url: &str) -> Result<(), ToolError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| ToolError::Refused("url is not a valid URL.".to_owned()))?;
    if parsed.username().is_empty() && parsed.password().is_none() {
        return Ok(());
    }
    Err(ToolError::Refused(
        "url holds a user name or password. Give the URL without them; the operator signs in \
         on the page itself."
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_004_a_url_with_credentials_is_refused() {
        assert!(without_credentials("https://10.0.0.5:8443/admin").is_ok());
        assert!(without_credentials("https://admin:secret@10.0.0.5").is_err());
        assert!(without_credentials("https://admin@10.0.0.5").is_err());
        let error = without_credentials("https://admin:secret@10.0.0.5")
            .map_or_else(|error| error.to_string(), |()| String::new());
        assert!(!error.contains("secret"), "{error}");
        assert!(without_credentials("not a url").is_err());
    }
}
