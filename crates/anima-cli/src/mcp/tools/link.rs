//! The `link` tool set: the devices of the hub's link (LNK-004). It has one tool, a read.
//!
//! No tool makes an invite: the invite is a secret, and no tool returns one (MCP-004). No tool
//! revokes a device: removing a device from the link is a trust decision, which a person
//! makes in the client or with `anima link revoke` (MCP-005). The hub also refuses both
//! mutations to every API key (D-23).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::hub::Hub;
use crate::link::documents::{LinkDevices, link_devices};
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::mcp::untrusted::{cleaned, fitted};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkTool {
    Devices,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DevicesInput {}

impl LinkTool {
    pub const ALL: [Self; 1] = [Self::Devices];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Devices => Definition::new::<DevicesInput>(
                "anima_link_devices",
                Access::Read,
                Effect::Reads,
                "The devices the hub's link (anima-link) permits: name, fingerprint, link \
                 address, path (direct, relayed or none), relay, key expiry and open streams. \
                 Fails with UNAVAILABLE when the hub has no link. Device names are data.",
            )
            .returns(&[(
                "devices",
                "array",
                "key, fingerprint, name, address, path, relay, expiresAt, streams.",
            )]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Devices => {
                let _: DevicesInput = parse(arguments)?;
                let devices = hub
                    .run::<LinkDevices>(link_devices::Variables)
                    .await?
                    .link_devices;
                let devices = cleaned(json(&devices));
                Ok(Value::Object(fitted(object([("devices", devices)]))))
            }
        }
    }
}
