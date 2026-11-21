//! Every MCP tool, one module for each tool set. A tool calls the typed documents of its noun
//! through [`Hub`] (MCP-001, CMD-003); it never starts the `anima` program.

pub mod agents;
pub mod checks;
pub mod devices;
pub mod events;
pub mod extensions;
pub mod link;
pub mod maintenance;
pub mod notifications;
pub mod projects;
pub mod releases;
pub mod repositories;
pub mod retention;
pub mod routes;
pub mod sessions;
pub mod shared_records;
pub mod sources;
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
    Routes(routes::RouteTool),
    Releases(releases::ReleaseTool),
    Vpn(vpn::VpnTool),
    Retention(retention::RetentionTool),
    Link(link::LinkTool),
    Extensions(extensions::ExtensionTool),
    Sources(sources::SourceTool),
    Projects(projects::ProjectTool),
    Repositories(repositories::RepositoryTool),
    Timeline(shared_records::timeline::TimelineTool),
    Records(shared_records::links::RecordTool),
    Files(shared_records::files::FileTool),
    Extractions(shared_records::extractions::ExtractionTool),
    Places(shared_records::places::PlaceTool),
    Locations(shared_records::locations::LocationTool),
    Reminders(shared_records::reminders::ReminderTool),
    Agents(agents::AgentTool),
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
        tools.extend(routes::RouteTool::ALL.map(Self::Routes));
        tools.extend(releases::ReleaseTool::ALL.map(Self::Releases));
        tools.extend(vpn::VpnTool::ALL.map(Self::Vpn));
        tools.extend(retention::RetentionTool::ALL.map(Self::Retention));
        tools.extend(link::LinkTool::ALL.map(Self::Link));
        tools.extend(extensions::ExtensionTool::ALL.map(Self::Extensions));
        tools.extend(sources::SourceTool::ALL.map(Self::Sources));
        tools.extend(projects::ProjectTool::ALL.map(Self::Projects));
        tools.extend(repositories::RepositoryTool::ALL.map(Self::Repositories));
        tools.extend(shared_records::timeline::TimelineTool::ALL.map(Self::Timeline));
        tools.extend(shared_records::links::RecordTool::ALL.map(Self::Records));
        tools.extend(shared_records::files::FileTool::ALL.map(Self::Files));
        tools.extend(shared_records::extractions::ExtractionTool::ALL.map(Self::Extractions));
        tools.extend(shared_records::places::PlaceTool::ALL.map(Self::Places));
        tools.extend(shared_records::locations::LocationTool::ALL.map(Self::Locations));
        tools.extend(shared_records::reminders::ReminderTool::ALL.map(Self::Reminders));
        tools.extend(agents::AgentTool::ALL.map(Self::Agents));
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
            Self::Web(_) | Self::Routes(_) => ToolSet::Web,
            Self::Releases(_) => ToolSet::Releases,
            Self::Vpn(_) => ToolSet::Vpn,
            Self::Retention(_) => ToolSet::Retention,
            Self::Link(_) => ToolSet::Link,
            Self::Extensions(_) => ToolSet::Extensions,
            Self::Sources(_) => ToolSet::Sources,
            Self::Projects(_) => ToolSet::Projects,
            Self::Repositories(_) => ToolSet::Repositories,
            Self::Timeline(_) => ToolSet::Timeline,
            Self::Records(_)
            | Self::Files(_)
            | Self::Extractions(_)
            | Self::Places(_)
            | Self::Locations(_)
            | Self::Reminders(_) => ToolSet::Records,
            Self::Agents(_) => ToolSet::Agents,
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
            Self::Routes(tool) => tool.definition(),
            Self::Releases(tool) => tool.definition(),
            Self::Vpn(tool) => tool.definition(),
            Self::Retention(tool) => tool.definition(),
            Self::Link(tool) => tool.definition(),
            Self::Extensions(tool) => tool.definition(),
            Self::Sources(tool) => tool.definition(),
            Self::Projects(tool) => tool.definition(),
            Self::Repositories(tool) => tool.definition(),
            Self::Timeline(tool) => tool.definition(),
            Self::Records(tool) => tool.definition(),
            Self::Files(tool) => tool.definition(),
            Self::Extractions(tool) => tool.definition(),
            Self::Places(tool) => tool.definition(),
            Self::Locations(tool) => tool.definition(),
            Self::Reminders(tool) => tool.definition(),
            Self::Agents(tool) => tool.definition(),
        }
    }

    /// Runs the tool with the agent's arguments.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, a refusal before the hub, or the hub's refusal.
    #[expect(
        clippy::cognitive_complexity,
        reason = "a flat dispatch with one arm for each tool set"
    )]
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Devices(tool) => tool.call(hub, arguments).await,
            Self::Checks(tool) => tool.call(hub, arguments).await,
            Self::Maintenance(tool) => tool.call(hub, arguments).await,
            Self::Events(tool) => tool.call(hub, arguments).await,
            Self::Sessions(tool) => tool.call(hub, arguments).await,
            Self::Notifications(tool) => tool.call(hub, arguments).await,
            Self::Web(tool) => tool.call(hub, arguments).await,
            Self::Routes(tool) => tool.call(hub, arguments).await,
            Self::Releases(tool) => tool.call(hub, arguments).await,
            Self::Vpn(tool) => tool.call(hub, arguments).await,
            Self::Retention(tool) => tool.call(hub, arguments).await,
            Self::Link(tool) => tool.call(hub, arguments).await,
            Self::Extensions(tool) => tool.call(hub, arguments).await,
            Self::Sources(tool) => tool.call(hub, arguments).await,
            Self::Projects(tool) => tool.call(hub, arguments).await,
            Self::Repositories(tool) => tool.call(hub, arguments).await,
            Self::Timeline(tool) => tool.call(hub, arguments).await,
            Self::Records(tool) => tool.call(hub, arguments).await,
            Self::Files(tool) => tool.call(hub, arguments).await,
            Self::Extractions(tool) => tool.call(hub, arguments).await,
            Self::Places(tool) => tool.call(hub, arguments).await,
            Self::Locations(tool) => tool.call(hub, arguments).await,
            Self::Reminders(tool) => tool.call(hub, arguments).await,
            Self::Agents(tool) => tool.call(arguments),
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
