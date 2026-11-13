//! The `checks` tool set: one check with its heartbeats, and the changes.

mod change;
mod config;
mod read;

use rmcp::model::JsonObject;
use serde_json::Value;

use crate::hub::Hub;
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckTool {
    Get,
    Create,
    Update,
    SetPaused,
    Delete,
}

const CHECK: [(&str, &str, &str); 1] = [(
    "check",
    "object",
    "The check: id, name, type, status, paused.",
)];

impl CheckTool {
    pub const ALL: [Self; 5] = [
        Self::Get,
        Self::Create,
        Self::Update,
        Self::SetPaused,
        Self::Delete,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Get => read::definition(),
            Self::Create => Definition::new::<change::CreateInput>(
                "anima_check_create",
                Access::Write,
                Effect::Adds,
                "Adds a check to a device. config is K1 CheckConfigInput with one key for the \
                 kind, such as {\"tcp\": {\"port\": 22}} or {\"http\": {\"url\": \
                 \"https://nas.lan\"}}. Fields that can hold a secret (HTTP headers, body, \
                 authentication; SNMP community and passwords) are refused: the operator adds \
                 those with `anima check create --from-file`. Returns the new check.",
            )
            .returns(&CHECK),
            Self::Update => Definition::new::<change::UpdateInput>(
                "anima_check_update",
                Access::Write,
                Effect::Sets,
                "Changes a check; only the fields given change. A new config replaces the \
                 whole config and follows the rules of anima_check_create; it is refused for \
                 every HTTP check (the hub does not say whether a request body is stored, and \
                 a body, headers or authentication would be dropped) and for every SNMP check, \
                 whose secrets it would drop. Returns the check.",
            )
            .returns(&CHECK),
            Self::SetPaused => Definition::new::<change::PausedInput>(
                "anima_check_set_paused",
                Access::Write,
                Effect::Sets,
                "Pauses a check (it stops running and alerting) or resumes it. Returns the \
                 check.",
            )
            .returns(&CHECK),
            Self::Delete => Definition::new::<change::DeleteInput>(
                "anima_check_delete",
                Access::Write,
                Effect::Removes,
                "Deletes a check and its heartbeats. It cannot be undone. Ask the operator \
                 first; confirm must be the check's exact name.",
            )
            .returns(&[("deleted", "string", "The deleted check's id.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, a refused secret field or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Get => read::get(hub, arguments).await,
            Self::Create => change::create(hub, arguments).await,
            Self::Update => change::update(hub, arguments).await,
            Self::SetPaused => change::set_paused(hub, arguments).await,
            Self::Delete => change::delete(hub, arguments).await,
        }
    }
}
