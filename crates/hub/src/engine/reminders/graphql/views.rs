//! The K1 types of the reminders part. A trigger and `unless` are JSON in the stored form of
//! K-E6 Section 2.

use async_graphql::{Enum, ID, InputObject, Json, SimpleObject};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::engine::reminders::{
    Draft, Fault, Firing, FiringState, Limits, StoredRule, trigger, trigger_json, unless,
    unless_json,
};

/// K1 `ReminderRule`.
#[derive(SimpleObject)]
#[graphql(name = "ReminderRule")]
pub(super) struct RuleView {
    id: ID,
    name: String,
    message: String,
    trigger: Json<Value>,
    unless: Option<Json<Value>>,
    cooldown_minutes: i32,
    daily_limit: i32,
    channel_ids: Vec<ID>,
    enabled: bool,
    revision: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_firing_at: Option<DateTime<Utc>>,
    fired_today: i32,
}

impl RuleView {
    /// The rule with the time of its newest firing and its count of the day.
    pub(super) fn new(stored: StoredRule, fired: Option<(DateTime<Utc>, i64)>) -> Self {
        let rule = stored.rule;
        Self {
            id: ID(rule.id.to_string()),
            name: rule.name,
            message: rule.message,
            trigger: Json(trigger_json(&rule.trigger)),
            unless: rule.unless.as_ref().map(|unless| Json(unless_json(unless))),
            cooldown_minutes: i32::try_from(rule.limits.cooldown_minutes).unwrap_or(i32::MAX),
            daily_limit: i32::try_from(rule.limits.daily_limit).unwrap_or(i32::MAX),
            channel_ids: stored
                .channels
                .iter()
                .map(|id| ID(id.to_string()))
                .collect(),
            enabled: stored.enabled,
            revision: rule.revision,
            created_at: stored.created_at,
            updated_at: rule.updated_at,
            last_firing_at: fired.map(|(last, _)| last),
            fired_today: fired.map_or(0, |(_, count)| i32::try_from(count).unwrap_or(i32::MAX)),
        }
    }
}

/// K1 `ReminderRuleInput`.
#[derive(InputObject)]
#[graphql(name = "ReminderRuleInput")]
pub(super) struct RuleInput {
    name: String,
    /// The text of the reminder; `{title}` is the title of what it is about.
    message: String,
    trigger: Json<Value>,
    unless: Option<Json<Value>>,
    #[graphql(default = 60)]
    cooldown_minutes: i32,
    #[graphql(default = 3)]
    daily_limit: i32,
    /// Empty or not given: each channel that is on by default.
    channel_ids: Option<Vec<ID>>,
    #[graphql(default = true)]
    enabled: bool,
}

impl RuleInput {
    /// The draft of the input, or the first field of its trigger, `unless` or channels to fix.
    pub(super) fn draft(self) -> Result<Draft, Fault> {
        let channels = self
            .channel_ids
            .unwrap_or_default()
            .iter()
            .map(|id| id.parse())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Fault {
                field: "channelIds",
                message: "Give notification channels that exist.",
            })?;
        Ok(Draft {
            name: self.name,
            message: self.message,
            trigger: trigger(&self.trigger.0)?,
            unless: self.unless.map(|json| unless(&json.0)).transpose()?,
            limits: Limits {
                cooldown_minutes: u32::try_from(self.cooldown_minutes).unwrap_or_default(),
                daily_limit: u32::try_from(self.daily_limit).unwrap_or_default(),
            },
            channels,
            enabled: self.enabled,
        })
    }
}

/// K1 `ReminderFiringState`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ReminderFiringState")]
pub(super) enum FiringStateView {
    Pending,
    Sent,
    Suppressed,
    Snoozed,
    Acknowledged,
}

impl From<FiringStateView> for FiringState {
    fn from(state: FiringStateView) -> Self {
        match state {
            FiringStateView::Pending => Self::Pending,
            FiringStateView::Sent => Self::Sent,
            FiringStateView::Suppressed => Self::Suppressed,
            FiringStateView::Snoozed => Self::Snoozed,
            FiringStateView::Acknowledged => Self::Acknowledged,
        }
    }
}

/// K1 `ReminderFiring`.
#[derive(SimpleObject)]
#[graphql(name = "ReminderFiring")]
pub(super) struct FiringView {
    id: ID,
    rule_id: ID,
    /// What the firing is about; a rule fires one time for one subject.
    subject: String,
    text: String,
    state: FiringStateView,
    /// Why a suppressed firing was not sent: `cooldown`, `daily_limit`, `rule_changed`,
    /// `topic_not_random` or `no_channel`.
    reason: Option<String>,
    due_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    snoozed_until: Option<DateTime<Utc>>,
    decided_at: Option<DateTime<Utc>>,
}

impl From<Firing> for FiringView {
    fn from(firing: Firing) -> Self {
        Self {
            id: ID(firing.id.to_string()),
            rule_id: ID(firing.rule.to_string()),
            subject: firing.subject,
            text: firing.text,
            state: match firing.state {
                FiringState::Pending => FiringStateView::Pending,
                FiringState::Sent => FiringStateView::Sent,
                FiringState::Suppressed => FiringStateView::Suppressed,
                FiringState::Snoozed => FiringStateView::Snoozed,
                FiringState::Acknowledged => FiringStateView::Acknowledged,
            },
            reason: firing.reason,
            due_at: firing.due_at,
            sent_at: firing.sent_at,
            snoozed_until: firing.snoozed_until,
            decided_at: firing.decided_at,
        }
    }
}
