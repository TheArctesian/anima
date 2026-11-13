//! The `notifications` tool set: channels by name and kind only, test messages, and which
//! channels a check tells. No tool makes or changes a channel: its address can hold a secret
//! (MCP-004).

use graphql_client::GraphQLQuery;
use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::checks::resolve::find;
use crate::checks::write::documents::{CheckNotificationsSet, check_notifications_set};
use crate::hub::Hub;
use crate::hub::scalars::DateTime;
use crate::mcp::arguments::{Page, json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::notifications::channel::resolve;
use crate::notifications::documents::{ChannelTest, channel_test};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/mcp/tools/notifications_overview.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct NotificationsOverview;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationTool {
    Overview,
    SendTest,
    CheckSetNotifications,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverviewInput {
    /// How many recent deliveries (default 25) and from where.
    #[serde(flatten)]
    page: Page,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestInput {
    /// The channel's id or exact name.
    channel: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckChannelsInput {
    /// The check's id, or its exact name together with `device`.
    check: String,
    /// The device's id or exact name; needed when `check` is a name.
    device: Option<String>,
    /// The channels the check tells (ids or exact names); this replaces the list. Empty: none.
    channels: Vec<String>,
    /// Tell again after each N DOWN heartbeats, 0 to 1000; 0 never (default 0).
    resend_every_beats: Option<i64>,
}

impl NotificationTool {
    pub const ALL: [Self; 3] = [Self::Overview, Self::SendTest, Self::CheckSetNotifications];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Overview => Definition::new::<OverviewInput>(
                "anima_notifications_overview",
                Access::Read,
                Effect::Reads,
                "The notification channels (id, name, kind such as EMAIL, NTFY, WEBHOOK, \
                 APPRISE; whether new checks use it; its last delivery), the certificate \
                 warning days and the latest deliveries (ok or not, HTTP code). Never an \
                 address or setting. Next: anima_check_set_notifications.",
            )
            .returns(&[
                ("channels", "array", "Every channel."),
                ("settings", "object", "certificateDays."),
                (
                    "deliveries",
                    "array",
                    "The latest deliveries, newest first.",
                ),
                (
                    "nextCursor",
                    "string",
                    "Give as `after` for older deliveries.",
                ),
            ]),
            Self::SendTest => Definition::new::<TestInput>(
                "anima_notification_send_test",
                Access::Write,
                Effect::Acts,
                "Sends a test message through a channel. Returns whether it was delivered and \
                 the HTTP code; the failure reason is left out because it can hold the \
                 channel's address (the operator sees it with `anima notify deliveries`).",
            )
            .returns(&[("delivery", "object", "The test delivery: ok, code, at.")]),
            Self::CheckSetNotifications => Definition::new::<CheckChannelsInput>(
                "anima_check_set_notifications",
                Access::Write,
                Effect::Sets,
                "Sets which channels a check tells when it goes DOWN or UP, and how often it \
                 repeats. Returns the check's notification settings.",
            )
            .returns(&[("notifications", "object", "resendEveryBeats and channels.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        match self {
            Self::Overview => overview(hub, arguments).await,
            Self::SendTest => send_test(hub, arguments).await,
            Self::CheckSetNotifications => check_channels(hub, arguments).await,
        }
    }
}

async fn overview(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: OverviewInput = parse(arguments)?;
    let variables = notifications_overview::Variables {
        first: Some(input.page.size()),
        after: input.page.after,
    };
    let answer = hub.run::<NotificationsOverview>(variables).await?;
    let page = answer.deliveries;
    let next = page
        .page_info
        .end_cursor
        .filter(|_| page.page_info.has_next_page);
    let deliveries: Vec<Value> = page.edges.iter().map(|edge| json(&edge.node)).collect();
    Ok(Value::Object(object([
        ("channels", json(&answer.notification_channels)),
        ("settings", json(&answer.notification_settings)),
        ("deliveries", Value::Array(deliveries)),
        ("nextCursor", json!(next)),
    ])))
}

async fn send_test(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: TestInput = parse(arguments)?;
    let id = resolve(hub, &input.channel).await?;
    let delivery = hub
        .run::<ChannelTest>(channel_test::Variables { id })
        .await?
        .send_test_notification;
    let shown = json!({
        "channelId": delivery.channel_id, "channelName": delivery.channel_name,
        "at": delivery.at, "ok": delivery.ok, "code": delivery.code,
    });
    Ok(Value::Object(object([("delivery", shown)])))
}

async fn check_channels(hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
    let input: CheckChannelsInput = parse(arguments)?;
    let check_id = find(hub, &input.check, input.device.as_deref()).await?;
    let mut channel_ids = Vec::with_capacity(input.channels.len());
    for channel in &input.channels {
        channel_ids.push(resolve(hub, channel).await?);
    }
    let variables = check_notifications_set::Variables {
        check_id,
        input: check_notifications_set::CheckNotificationsInput {
            channel_ids,
            resend_every_beats: input.resend_every_beats,
        },
    };
    let notifications = hub
        .run::<CheckNotificationsSet>(variables)
        .await?
        .set_check_notifications;
    Ok(Value::Object(object([(
        "notifications",
        json(&notifications),
    )])))
}
