//! What the agent sees of one tool (MCP-009, MCP-011): its name, a description, the input and
//! output schemas, and the hints. Each tool set builds its definitions from these parts.

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde_json::{Value, json};

use super::catalogue::Access;

/// What a tool does to the hub, for the hints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Reads only.
    Reads,
    /// Adds something new; a second call adds another.
    Adds,
    /// Sets a value; a second call with the same arguments changes nothing more.
    Sets,
    /// Does something each time it is called (a Wake-on-LAN packet, a test message).
    Acts,
    /// Removes something or stops it for good.
    Removes,
}

/// One tool, before the server turns it into the MCP form.
#[derive(Clone, Debug)]
pub struct Definition {
    /// `anima_<noun>_<verb>` (MCP-011).
    pub name: &'static str,
    pub access: Access,
    pub effect: Effect,
    /// For an agent that has never seen Anima: what it returns, when to use it, what next.
    pub description: &'static str,
    pub input: Arc<JsonObject>,
    pub output: Arc<JsonObject>,
}

impl Definition {
    /// A definition whose input is the type `I`.
    #[must_use]
    pub fn new<I: JsonSchema + 'static>(
        name: &'static str,
        access: Access,
        effect: Effect,
        description: &'static str,
    ) -> Self {
        let input = Tool::new(name, description, JsonObject::new())
            .with_input_schema::<I>()
            .input_schema;
        Self {
            name,
            access,
            effect,
            description,
            input,
            output: Arc::new(JsonObject::new()),
        }
    }

    /// The same definition with an output schema of these top-level fields (name, JSON type,
    /// meaning).
    #[must_use]
    pub fn returns(mut self, fields: &[(&str, &str, &str)]) -> Self {
        self.output = Arc::new(output(fields));
        self
    }

    /// The MCP tool (MCP-009): annotations from the effect, `openWorldHint` false.
    #[must_use]
    pub fn tool(&self) -> Tool {
        let reads = self.effect == Effect::Reads;
        let annotations = ToolAnnotations::new()
            .read_only(reads)
            .destructive(self.effect == Effect::Removes)
            .idempotent(matches!(
                self.effect,
                Effect::Reads | Effect::Sets | Effect::Removes
            ))
            .open_world(false);
        Tool::new(self.name, self.description, self.input.clone())
            .with_raw_output_schema(self.output.clone())
            .with_annotations(annotations)
    }
}

/// An object schema with these properties; every result may also say it was cut
/// (`truncated`, `note`).
fn output(fields: &[(&str, &str, &str)]) -> JsonObject {
    let mut properties = serde_json::Map::new();
    for (name, kind, meaning) in fields {
        properties.insert(
            (*name).to_owned(),
            json!({ "type": kind, "description": meaning }),
        );
    }
    properties.insert(
        "truncated".to_owned(),
        json!({ "type": "boolean", "description": "True when the result was cut to fit." }),
    );
    properties.insert(
        "note".to_owned(),
        json!({ "type": "string", "description": "Why it was cut and how to get the rest." }),
    );
    let schema = json!({ "type": "object", "properties": Value::Object(properties) });
    match schema {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "only the schema of the type is used")]
    struct Lookup {
        /// The device's ID or exact name.
        device: String,
    }

    #[test]
    fn mcp_009_a_tool_has_schemas_and_every_hint() {
        let tool = Definition::new::<Lookup>(
            "anima_device_get",
            Access::Read,
            Effect::Reads,
            "One device.",
        )
        .returns(&[("device", "object", "The device.")])
        .tool();
        let shown = serde_json::to_value(&tool).unwrap_or_default();
        assert_eq!(shown["inputSchema"]["type"], "object");
        assert!(
            shown["inputSchema"]["properties"]["device"]["description"]
                .as_str()
                .is_some_and(|text| text.contains("exact name"))
        );
        assert_eq!(
            shown["outputSchema"]["properties"]["device"]["type"],
            "object"
        );
        let hints = &shown["annotations"];
        assert_eq!(hints["readOnlyHint"], true);
        assert_eq!(hints["destructiveHint"], false);
        assert_eq!(hints["idempotentHint"], true);
        assert_eq!(hints["openWorldHint"], false);
    }

    #[test]
    fn mcp_009_a_removing_tool_is_destructive() {
        let tool = Definition::new::<Lookup>("x", Access::Login, Effect::Removes, "x").tool();
        let hints = tool.annotations.unwrap_or_default();
        assert_eq!(hints.read_only_hint, Some(false));
        assert_eq!(hints.destructive_hint, Some(true));
    }
}
