//! The `maintenance` tool set: windows in which devices, checks or tags raise no alert.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::hub::Hub;
use crate::maintenance::documents::{
    MaintenanceCreate, MaintenanceDelete, MaintenanceList, MaintenanceSetActive, MaintenanceUpdate,
    maintenance_create, maintenance_delete, maintenance_list, maintenance_set_active,
    maintenance_update,
};
use crate::maintenance::fields::INPUT;
use crate::maintenance::list::find;
use crate::mcp::arguments::{Page, confirmed, json, k1_input, object, parse, present, short};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceTool {
    List,
    Create,
    Update,
    Delete,
    SetActive,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    /// Add every schedule field.
    #[serde(default)]
    detail: bool,
    #[serde(flatten)]
    page: Page,
}

/// The fields of K1 `MaintenanceWindowInput`. The kind decides which schedule fields count.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowFields {
    name: Option<String>,
    /// `MANUAL`, `SINGLE`, `RECURRING_INTERVAL`, `RECURRING_WEEKDAY` or `RECURRING_DAY_OF_MONTH`.
    kind: Option<String>,
    /// The time zone the times are local to, such as Europe/Berlin.
    timezone: Option<String>,
    /// `SINGLE`: the start (RFC 3339).
    starts_at: Option<String>,
    /// `SINGLE`: the end (RFC 3339).
    ends_at: Option<String>,
    /// `RECURRING_INTERVAL`: the first start (RFC 3339).
    first_start_at: Option<String>,
    /// `RECURRING_INTERVAL`: days between starts.
    every_days: Option<i64>,
    /// `RECURRING_WEEKDAY`: such as `["MON", "WED"]`.
    weekdays: Option<Vec<String>>,
    /// `RECURRING_DAY_OF_MONTH`: 1 to 31, or -1 for the last day.
    days_of_month: Option<Vec<i64>>,
    /// `RECURRING_*`: the local start, HH:MM.
    start_time: Option<String>,
    /// `RECURRING_*`: how long it lasts.
    duration_minutes: Option<i64>,
    /// What it covers: `[{"type": "DEVICE", "target": "<id>"}]`; type is `DEVICE`, `CHECK` or `TAG`.
    targets: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    /// The window's id or exact name. Only the fields given change.
    window: String,
    #[serde(flatten)]
    fields: WindowFields,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    /// The window's id or exact name.
    window: String,
    /// The window's exact name, as the operator confirmed it.
    confirm: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveInput {
    /// The MANUAL window's id or exact name.
    window: String,
    /// true starts it now, false stops it.
    active: bool,
}

const WINDOW: [(&str, &str, &str); 1] = [("window", "object", "id, name, kind, activeNow.")];

impl MaintenanceTool {
    pub const ALL: [Self; 5] = [
        Self::List,
        Self::Create,
        Self::Update,
        Self::Delete,
        Self::SetActive,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_maintenance_list",
                Access::Read,
                Effect::Reads,
                "Lists maintenance windows: times when devices, checks or tags raise no \
                 alert. Returns id, name, kind, whether it is active now and its targets; \
                 detail adds the schedule. Next: anima_maintenance_set_active for a manual \
                 window.",
            )
            .returns(&[
                ("windows", "array", "The windows of this page."),
                ("nextCursor", "string", "Give as `after` for the next page."),
            ]),
            Self::Create => Definition::new::<WindowFields>(
                "anima_maintenance_create",
                Access::Write,
                Effect::Adds,
                "Adds a maintenance window. name, kind, timezone and targets are needed; the \
                 kind decides the schedule fields. Returns the window.",
            )
            .returns(&WINDOW),
            Self::Update => Definition::new::<UpdateInput>(
                "anima_maintenance_update",
                Access::Write,
                Effect::Sets,
                "Changes a maintenance window; only the fields given change. Returns it.",
            )
            .returns(&WINDOW),
            Self::Delete => Definition::new::<DeleteInput>(
                "anima_maintenance_delete",
                Access::Write,
                Effect::Removes,
                "Deletes a maintenance window. Ask the operator first; confirm must be the \
                 window's exact name.",
            )
            .returns(&[("deleted", "string", "The deleted window's id.")]),
            Self::SetActive => Definition::new::<ActiveInput>(
                "anima_maintenance_set_active",
                Access::Write,
                Effect::Sets,
                "Starts or stops a MANUAL maintenance window now. Returns the window.",
            )
            .returns(&WINDOW),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::List => list(hub, arguments).await,
            Self::Create => create(hub, arguments).await,
            Self::Update => update(hub, arguments).await,
            Self::Delete => delete(hub, arguments).await,
            Self::SetActive => set_active(hub, arguments).await,
        }
    }
}

async fn list(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ListInput = parse(arguments)?;
    let windows = hub
        .run::<MaintenanceList>(maintenance_list::Variables)
        .await?
        .maintenance_windows;
    let keys = ["id", "name", "kind", "activeNow", "targets"];
    let all = windows
        .iter()
        .map(|window| short(json(window), &keys, input.detail))
        .collect();
    let (windows, next) = input.page.of(all);
    Ok(Value::Object(object([
        ("windows", Value::Array(windows)),
        ("nextCursor", json!(next)),
    ])))
}

async fn create(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let fields: WindowFields = parse(arguments)?;
    let input = k1_input(present(&fields), "window")?;
    let window = hub
        .run::<MaintenanceCreate>(maintenance_create::Variables { input })
        .await?
        .create_maintenance_window;
    Ok(Value::Object(object([("window", json(&window))])))
}

async fn update(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: UpdateInput = parse(arguments)?;
    let window = find(hub, &input.window).await?;
    let mut base: Map<String, Value> = match json(&window) {
        Value::Object(fields) => fields
            .into_iter()
            .filter(|(key, value)| INPUT.contains(&key.as_str()) && !value.is_null())
            .collect(),
        _ => Map::new(),
    };
    base.extend(present(&input.fields));
    let variables = maintenance_update::Variables {
        id: window.id,
        input: k1_input(base, "window")?,
    };
    let window = hub
        .run::<MaintenanceUpdate>(variables)
        .await?
        .update_maintenance_window;
    Ok(Value::Object(object([("window", json(&window))])))
}

async fn delete(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: DeleteInput = parse(arguments)?;
    let window = find(hub, &input.window).await?;
    confirmed(&input.confirm, &window.name, "maintenance window")?;
    let deleted = hub
        .run::<MaintenanceDelete>(maintenance_delete::Variables { id: window.id })
        .await?
        .delete_maintenance_window;
    Ok(Value::Object(object([("deleted", json!(deleted))])))
}

async fn set_active(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: ActiveInput = parse(arguments)?;
    let window = find(hub, &input.window).await?;
    let variables = maintenance_set_active::Variables {
        id: window.id,
        active: input.active,
    };
    let window = hub
        .run::<MaintenanceSetActive>(variables)
        .await?
        .set_manual_window_active;
    Ok(Value::Object(object([("window", json(&window))])))
}
