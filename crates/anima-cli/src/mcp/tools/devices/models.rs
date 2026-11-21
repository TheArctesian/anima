//! The `models` tool set: the model library, read only (SEL-005, CMD-030). Names and tags
//! come from Hugging Face repositories: the server cleans and caps each string (MCP-008).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse, short};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::model_library::documents::{ModelList, ModelShow, model_list, model_show};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelTool {
    List,
    Show,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShowInput {
    /// The model's name, an alias, or name:tag.
    name: String,
    /// Add the file list of each variant with its SHA-256 values.
    #[serde(default)]
    detail: bool,
}

const VARIANT_KEYS: [&str; 6] = [
    "address",
    "tag",
    "format",
    "quantisation",
    "sourceRepository",
    "sizeBytes",
];

impl ModelTool {
    pub const ALL: [Self; 2] = [Self::List, Self::Show];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_model_list",
                Access::Read,
                Effect::Reads,
                "The models of the library in name order: name, family, parameter count, \
                 context length, capability tags, licence, aliases, and each variant by its \
                 address name:tag with its format, quantisation and size. A tag says what \
                 a model can do, not that a device runs it.",
            )
            .returns(&[("models", "array", "The models of the library.")]),
            Self::Show => Definition::new::<ShowInput>(
                "anima_model_show",
                Access::Read,
                Effect::Reads,
                "One model by its name, alias or name:tag: its facts (layers, hidden size, \
                 heads), its publisher and source repositories, and its variants; with \
                 detail, each file with its size and SHA-256 value. Null when there is no \
                 such model.",
            )
            .returns(&[("model", "object", "The model; null when unknown.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => list(hub, arguments).await,
            Self::Show => show(hub, arguments).await,
        }
    }
}

async fn list(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let _: ListInput = parse(arguments)?;
    let models = hub
        .run::<ModelList>(model_list::Variables)
        .await?
        .library_models;
    Ok(Value::Object(object([("models", json(&models))])))
}

async fn show(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ShowInput = parse(arguments)?;
    let answer = hub
        .run::<ModelShow>(model_show::Variables { name: input.name })
        .await?;
    let mut model = json(&answer.library_model);
    if let Some(variants) = model.get_mut("variants").and_then(Value::as_array_mut) {
        for variant in variants.iter_mut() {
            *variant = short(variant.take(), &VARIANT_KEYS, input.detail);
        }
    }
    Ok(json!({ "model": model }))
}
