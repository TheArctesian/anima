//! The arguments of the reminder tools, as an agent gives them.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoInput {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdInput {
    /// The rule's id.
    pub(super) id: String,
}

/// A whole rule, as K1 `ReminderRuleInput`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuleInput {
    /// One line of 1 to 80 characters.
    pub(super) name: String,
    /// The text of the reminder, one line of at most 200 characters; `{title}` is the title of
    /// what it is about.
    pub(super) message: String,
    /// One trigger: `{"type":"time","at":"09:00","days":["mon"],"zone":"Europe/Amsterdam"}`,
    /// `{"type":"time","once":"2026-12-01T09:00:00Z"}`,
    /// `{"type":"place","place":"<id>","on":"arrive"}`,
    /// `{"type":"source_event","source":"strava","kind":"workout","contains":"run"}`,
    /// `{"type":"source_state","account":null,"state":"needs_reconnect"}`,
    /// `{"type":"threshold","key":"health.sleep_hours","is":"below","value":6}` or
    /// `{"type":"expiry","subject":"documents.passport","days_before":[21,7]}`.
    pub(super) trigger: JsonObject,
    /// One condition that stops the reminder: `{"record_today":{"source":"strava",
    /// "kind":"workout"}}` or `{"measure":{"key":"health.steps","is":"above","value":8000}}`.
    pub(super) unless: Option<JsonObject>,
    /// Minutes between two reminders of the rule, 1 to 10080; 60 when left out.
    pub(super) cooldown_minutes: Option<i64>,
    /// The most reminders of the rule in one day, 1 to 100; 3 when left out.
    pub(super) daily_limit: Option<i64>,
    /// Notification channels by id; empty or left out means each channel that is on by default.
    pub(super) channel_ids: Option<Vec<String>>,
    /// False makes the rule off; true when left out.
    pub(super) enabled: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetInput {
    /// The rule's id.
    pub(super) id: String,
    /// The whole rule as it must be after the change.
    pub(super) rule: RuleInput,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnabledInput {
    /// The rule's id.
    pub(super) id: String,
    pub(super) enabled: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FiringsInput {
    /// Only the firings of this rule.
    pub(super) rule: Option<String>,
    /// `PENDING`, `SENT`, `SUPPRESSED`, `SNOOZED` or `ACKNOWLEDGED`.
    pub(super) state: Option<String>,
    /// The most firings, 1 to 200; 200 when left out.
    pub(super) first: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnoozeInput {
    /// The firing's id.
    pub(super) firing: String,
    /// Minutes until it is sent again, 1 to 10080.
    pub(super) minutes: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AckInput {
    /// The firing's id.
    pub(super) firing: String,
}
