//! Every MCP tool, one module for each tool set. A tool calls the typed documents of its noun
//! through [`Hub`] (MCP-001, CMD-003); it never starts the `anima` program.

pub mod devices;

use rmcp::model::JsonObject;
use serde_json::Value;

use super::catalogue::{Grant, ToolSet};
use super::definition::Definition;
use super::outcome::ToolError;
use crate::hub::Hub;

/// One tool of one set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Devices(devices::DeviceTool),
}

impl Tool {
    /// Every tool, set by set.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut tools: Vec<Self> = Vec::new();
        tools.extend(devices::DeviceTool::ALL.map(Self::Devices));
        tools
    }

    #[must_use]
    pub const fn set(self) -> ToolSet {
        match self {
            Self::Devices(_) => ToolSet::Devices,
        }
    }

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Devices(tool) => tool.definition(),
        }
    }

    /// Runs the tool with the agent's arguments.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments, a refusal before the hub, or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Devices(tool) => tool.call(hub, arguments).await,
        }
    }
}

/// The tools `grant` offers, with their definitions (MCP-002, MCP-003).
#[must_use]
pub fn offered(grant: &Grant) -> Vec<(Tool, Definition)> {
    Tool::all()
        .into_iter()
        .map(|tool| (tool, tool.definition()))
        .filter(|(tool, definition)| grant.offers(tool.set(), definition.access))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::hub::TokenKind;
    use crate::mcp::catalogue::Access;

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
    fn mcp_003_an_api_key_with_write_gets_no_login_tool() {
        let grant = Grant {
            token: TokenKind::ApiKey,
            toolsets: Vec::new(),
            write: ToolSet::ALL.to_vec(),
        };
        let offered = offered(&grant);
        assert!(
            offered
                .iter()
                .all(|(_, definition)| definition.access != Access::Login)
        );
        assert!(
            offered
                .iter()
                .any(|(_, definition)| definition.access == Access::Write)
        );
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
