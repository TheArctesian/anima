//! Every MCP tool, one module for each tool set. A tool calls the typed documents of its noun
//! through [`Hub`] (MCP-001, CMD-003); it never starts the `anima` program.

pub mod checks;
pub mod devices;
pub mod events;
pub mod maintenance;
pub mod notifications;
pub mod releases;
pub mod retention;
pub mod sessions;
pub mod vpn;
pub mod web;

use rmcp::model::JsonObject;
use serde_json::Value;

use super::catalogue::{Kind, ToolSet};
use super::definition::Definition;
use super::outcome::ToolError;
use crate::hub::Hub;

/// One tool of one set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Devices(devices::DeviceTool),
    Checks(checks::CheckTool),
    Maintenance(maintenance::MaintenanceTool),
    Events(events::EventTool),
    Sessions(sessions::SessionTool),
    Notifications(notifications::NotificationTool),
    Web(web::WebTool),
    Releases(releases::ReleaseTool),
    Vpn(vpn::VpnTool),
    Retention(retention::RetentionTool),
}

impl Tool {
    /// Every tool, set by set.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut tools: Vec<Self> = Vec::new();
        tools.extend(devices::DeviceTool::ALL.map(Self::Devices));
        tools.extend(checks::CheckTool::ALL.map(Self::Checks));
        tools.extend(maintenance::MaintenanceTool::ALL.map(Self::Maintenance));
        tools.extend(events::EventTool::ALL.map(Self::Events));
        tools.extend(sessions::SessionTool::ALL.map(Self::Sessions));
        tools.extend(notifications::NotificationTool::ALL.map(Self::Notifications));
        tools.extend(web::WebTool::ALL.map(Self::Web));
        tools.extend(releases::ReleaseTool::ALL.map(Self::Releases));
        tools.extend(vpn::VpnTool::ALL.map(Self::Vpn));
        tools.extend(retention::RetentionTool::ALL.map(Self::Retention));
        tools
    }

    #[must_use]
    pub const fn set(self) -> ToolSet {
        match self {
            Self::Devices(_) => ToolSet::Devices,
            Self::Checks(_) => ToolSet::Checks,
            Self::Maintenance(_) => ToolSet::Maintenance,
            Self::Events(_) => ToolSet::Events,
            Self::Sessions(_) => ToolSet::Sessions,
            Self::Notifications(_) => ToolSet::Notifications,
            Self::Web(_) => ToolSet::Web,
            Self::Releases(_) => ToolSet::Releases,
            Self::Vpn(_) => ToolSet::Vpn,
            Self::Retention(_) => ToolSet::Retention,
        }
    }

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Devices(tool) => tool.definition(),
            Self::Checks(tool) => tool.definition(),
            Self::Maintenance(tool) => tool.definition(),
            Self::Events(tool) => tool.definition(),
            Self::Sessions(tool) => tool.definition(),
            Self::Notifications(tool) => tool.definition(),
            Self::Web(tool) => tool.definition(),
            Self::Releases(tool) => tool.definition(),
            Self::Vpn(tool) => tool.definition(),
            Self::Retention(tool) => tool.definition(),
        }
    }

    /// Runs the tool with the agent's arguments.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, a refusal before the hub, or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Devices(tool) => tool.call(hub, arguments).await,
            Self::Checks(tool) => tool.call(hub, arguments).await,
            Self::Maintenance(tool) => tool.call(hub, arguments).await,
            Self::Events(tool) => tool.call(hub, arguments).await,
            Self::Sessions(tool) => tool.call(hub, arguments).await,
            Self::Notifications(tool) => tool.call(hub, arguments).await,
            Self::Web(tool) => tool.call(hub, arguments).await,
            Self::Releases(tool) => tool.call(hub, arguments).await,
            Self::Vpn(tool) => tool.call(hub, arguments).await,
            Self::Retention(tool) => tool.call(hub, arguments).await,
        }
    }
}

/// One tool of the catalogue with its definition.
#[derive(Clone, Debug)]
pub struct Entry {
    pub tool: Tool,
    pub set: ToolSet,
    /// `anima_<noun>_<verb>`, as in the definition.
    pub name: &'static str,
    pub definition: Definition,
}

impl Entry {
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.definition.access.kind()
    }
}

/// Every tool with its definition, set by set.
#[must_use]
pub fn catalogue() -> Vec<Entry> {
    Tool::all()
        .into_iter()
        .map(|tool| {
            let definition = tool.definition();
            Entry {
                tool,
                set: tool.set(),
                name: definition.name,
                definition,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn mcp_011_every_name_is_anima_noun_verb_and_unique() {
        let mut seen = HashSet::new();
        for tool in Tool::all() {
            let name = tool.definition().name;
            assert!(
                name.starts_with("anima_") && name.matches('_').count() >= 2,
                "{name}"
            );
            assert!(seen.insert(name), "{name} twice");
        }
    }

    #[test]
    fn mcp_009_every_tool_has_a_description_and_an_object_output_schema() {
        for tool in Tool::all() {
            let definition = tool.definition();
            assert!(definition.description.len() > 40, "{}", definition.name);
            assert_eq!(
                definition.output.get("type").and_then(Value::as_str),
                Some("object"),
                "{}",
                definition.name
            );
        }
    }
}
