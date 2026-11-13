//! The `retention` tool set: how long the hub keeps heartbeats, metrics, events and
//! deliveries.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hub::Hub;
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::retention::documents::{RetentionList, RetentionSet, retention_list, retention_set};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionTool {
    List,
    Set,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {}

/// K1 `RetentionSubject`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubjectInput {
    Heartbeats,
    Metrics,
    Events,
    Deliveries,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetInput {
    /// What the period is for.
    subject: SubjectInput,
    /// Days to keep, between the subject's `minDays` and `maxDays` (see `anima_retention_list`).
    days: i64,
}

impl RetentionTool {
    pub const ALL: [Self; 2] = [Self::List, Self::Set];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_retention_list",
                Access::Read,
                Effect::Reads,
                "How many days the hub keeps heartbeats, metrics, events and deliveries, with \
                 the lowest, highest and default number of days for each.",
            )
            .returns(&[(
                "periods",
                "array",
                "subject, days, minDays, maxDays, defaultDays.",
            )]),
            Self::Set => Definition::new::<SetInput>(
                "anima_retention_set",
                Access::Login,
                Effect::Sets,
                "Sets how many days the hub keeps one kind of record. A shorter period deletes \
                 older records at the next clean-up; ask the operator first. Returns the period.",
            )
            .returns(&[(
                "period",
                "object",
                "subject, days, minDays, maxDays, defaultDays.",
            )]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => {
                let _: ListInput = parse(arguments)?;
                let periods = hub
                    .run::<RetentionList>(retention_list::Variables)
                    .await?
                    .retention;
                Ok(Value::Object(object([("periods", json(&periods))])))
            }
            Self::Set => {
                let input: SetInput = parse(arguments)?;
                let subject = serde_json::from_value(json(&input.subject))
                    .map_err(|error| ToolError::Refused(format!("subject: {error}.")))?;
                let variables = retention_set::Variables {
                    subject,
                    days: input.days,
                };
                let period = hub.run::<RetentionSet>(variables).await?.set_retention;
                Ok(Value::Object(object([("period", json(&period))])))
            }
        }
    }
}
