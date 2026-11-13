//! `anima_check_get`: one check, its settings without a secret, and its heartbeats.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::checks::read::documents::{CheckGet, CheckHeartbeats, check_get, check_heartbeats};
use crate::checks::resolve::find;
use crate::hub::Hub;
use crate::mcp::arguments::{Page, json, object, parse, short};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetInput {
    /// The check's id, or its exact name together with `device`.
    pub check: String,
    /// The device's id or exact name; needed when `check` is a name.
    pub device: Option<String>,
    /// Add the retry settings, parent, notifications and certificate or domain days.
    #[serde(default)]
    detail: bool,
    /// How many heartbeats (default 25) and from where.
    #[serde(flatten)]
    page: Page,
}

/// `anima_check_get`.
pub fn definition() -> Definition {
    Definition::new::<GetInput>(
        "anima_check_get",
        Access::Read,
        Effect::Reads,
        "One check (a probe the hub runs against a device: ping, TCP, TLS, HTTP, DNS, \
         push, group, Docker, SNMP, Tailscale ping, systemd, domain expiry) by id, or \
         by exact name with its device. Returns its status, settings (never a secret), \
         24-hour uptime and its latest heartbeats (status, response time, message). \
         Find check ids with anima_device_get.",
    )
    .returns(&[
        ("check", "object", "The check."),
        (
            "heartbeats",
            "array",
            "The latest heartbeats, newest first.",
        ),
        (
            "nextCursor",
            "string",
            "Give as `after` for older heartbeats.",
        ),
    ])
}

const SHORT_KEYS: [&str; 10] = [
    "id",
    "name",
    "type",
    "status",
    "paused",
    "device",
    "config",
    "intervalSeconds",
    "lastHeartbeat",
    "uptime",
];

pub async fn get(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: GetInput = parse(arguments)?;
    let id = find(hub, &input.check, input.device.as_deref()).await?;
    let check = current(hub, &id).await?;
    let variables = check_heartbeats::Variables {
        id,
        first: Some(input.page.size()),
        after: input.page.after,
    };
    let heartbeats = hub
        .run::<CheckHeartbeats>(variables)
        .await?
        .check
        .map(|check| check.heartbeats);
    let (beats, next) = heartbeats.map_or((Vec::new(), None), |page| {
        let next = page
            .page_info
            .end_cursor
            .filter(|_| page.page_info.has_next_page);
        (
            page.edges.iter().map(|edge| json(&edge.node)).collect(),
            next,
        )
    });
    Ok(Value::Object(object([
        ("check", short(check, &SHORT_KEYS, input.detail)),
        ("heartbeats", Value::Array(beats)),
        ("nextCursor", json!(next)),
    ])))
}

/// The check `id` as K1 JSON without its push path: the path holds the push token (MCP-004).
///
/// # Errors
/// [`ToolError`] when the hub has no such check, or refuses.
pub async fn current(hub: &Hub, id: &str) -> Result<Value, ToolError> {
    let check = hub
        .run::<CheckGet>(check_get::Variables { id: id.to_owned() })
        .await?
        .check
        .ok_or_else(|| {
            ToolError::Refused(format!(
                "No check has the id {id}. Call anima_device_get to list a device's checks."
            ))
        })?;
    let mut check = json(&check);
    if let Some(config) = check.get_mut("config").and_then(Value::as_object_mut) {
        config.remove("pushPath");
    }
    Ok(check)
}
