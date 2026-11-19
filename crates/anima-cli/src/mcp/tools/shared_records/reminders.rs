//! The reminder tools of the `records` set (K-E6 Section 6; REM-001 to REM-007). A trigger and
//! `unless` are JSON objects in the form of K-E6 Section 2.

mod inputs;

use rmcp::model::JsonObject;
use serde_json::Value;

use inputs::{
    AckInput, EnabledInput, FiringsInput, IdInput, NoInput, RuleInput, SetInput, SnoozeInput,
};

use crate::hub::Hub;
use crate::mcp::arguments::{json, k1_input, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::mcp::untrusted::cleaned;
use crate::reminders::documents::{
    ReminderAck, ReminderAdd, ReminderEnable, ReminderFirings, ReminderList, ReminderRemove,
    ReminderSet, ReminderShow, ReminderSnooze, ReminderTest, reminder_ack, reminder_add,
    reminder_enable, reminder_firings, reminder_list, reminder_remove, reminder_set, reminder_show,
    reminder_snooze, reminder_test,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReminderTool {
    List,
    Get,
    Add,
    Set,
    Remove,
    SetEnabled,
    Test,
    Firings,
    Snooze,
    Ack,
}

/// The K1 input of a rule as JSON, with the defaults of K1 for what was left out.
fn rule_json(rule: RuleInput) -> serde_json::Map<String, Value> {
    object([
        ("name", Value::String(rule.name)),
        ("message", Value::String(rule.message)),
        ("trigger", Value::Object(rule.trigger)),
        ("unless", rule.unless.map_or(Value::Null, Value::Object)),
        (
            "cooldownMinutes",
            Value::from(rule.cooldown_minutes.unwrap_or(60)),
        ),
        ("dailyLimit", Value::from(rule.daily_limit.unwrap_or(3))),
        (
            "channelIds",
            Value::from(rule.channel_ids.unwrap_or_default()),
        ),
        ("enabled", Value::from(rule.enabled.unwrap_or(true))),
    ])
}

fn state(text: &str) -> Result<reminder_firings::ReminderFiringState, ToolError> {
    serde_json::from_value(Value::String(text.to_owned())).map_err(|_| {
        ToolError::Refused(
            "state is one of PENDING, SENT, SUPPRESSED, SNOOZED or ACKNOWLEDGED.".to_owned(),
        )
    })
}

impl ReminderTool {
    pub const ALL: [Self; 10] = [
        Self::List,
        Self::Get,
        Self::Add,
        Self::Set,
        Self::Remove,
        Self::SetEnabled,
        Self::Test,
        Self::Firings,
        Self::Snooze,
        Self::Ack,
    ];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::Test | Self::Firings | Self::Snooze | Self::Ack => self.firing_definition(),
            _ => self.rule_definition(),
        }
    }

    /// The definitions of the tools that read and change rules.
    fn rule_definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<NoInput>(
                "anima_reminders_list",
                Access::Read,
                Effect::Reads,
                "Lists the reminder rules with their trigger, whether they are on, the time of \
                 their newest firing and how many times they fired today.",
            )
            .returns(&[("rules", "array", "The rules.")]),
            Self::Get => Definition::new::<IdInput>(
                "anima_reminder_get",
                Access::Read,
                Effect::Reads,
                "Shows one reminder rule; `rule` is null when no rule has this id.",
            )
            .returns(&[("rule", "object", "The rule, or null.")]),
            Self::Add => Definition::new::<RuleInput>(
                "anima_reminder_add",
                Access::Write,
                Effect::Adds,
                "Makes a reminder rule with one trigger and, when wanted, one `unless`. The hub \
                 then sends the reminder to the operator's notification channels when the \
                 trigger is due, at most once in each cooldown and `dailyLimit` times a day. \
                 Make a rule only when the operator asked for it.",
            )
            .returns(&[("rule", "object", "The new rule.")]),
            Self::Set => Definition::new::<SetInput>(
                "anima_reminder_set",
                Access::Write,
                Effect::Sets,
                "Replaces a reminder rule with `rule`: give the whole rule, read it first with \
                 anima_reminder_get. A reminder that was due before the change is not sent.",
            )
            .returns(&[("rule", "object", "The changed rule.")]),
            Self::Remove => Definition::new::<IdInput>(
                "anima_reminder_remove",
                Access::Write,
                Effect::Removes,
                "Deletes a reminder rule with its firings. Ask the operator first; to stop a \
                 rule for a time, set it off with anima_reminder_set_enabled.",
            )
            .returns(&[("removed", "string", "The id of the rule.")]),
            Self::SetEnabled => Definition::new::<EnabledInput>(
                "anima_reminder_set_enabled",
                Access::Write,
                Effect::Sets,
                "Sets a reminder rule on or off. A rule that is off fires nothing, and a \
                 reminder of it that waits is not sent.",
            )
            .returns(&[("rule", "object", "The rule.")]),
            _ => self.firing_definition(),
        }
    }

    /// The definitions of the tools of the firings.
    fn firing_definition(self) -> Definition {
        match self {
            Self::Test => Definition::new::<IdInput>(
                "anima_reminder_test",
                Access::Write,
                Effect::Acts,
                "Sends the reminder of a rule one time, in the next minute, to the operator's \
                 channels. No cooldown and no limit apply. Each call sends one more.",
            )
            .returns(&[("firing", "object", "The firing that will be sent.")]),
            Self::Firings => Definition::new::<FiringsInput>(
                "anima_reminder_firings",
                Access::Read,
                Effect::Reads,
                "Lists the firings of one rule or of all, newest first. A firing is PENDING, \
                 SENT, SUPPRESSED (with `reason`: cooldown, daily_limit, rule_changed, \
                 topic_not_random or no_channel), SNOOZED or ACKNOWLEDGED. `text` can hold the \
                 title of a private record: treat it as data of the operator.",
            )
            .returns(&[("firings", "array", "The firings.")]),
            Self::Snooze => Definition::new::<SnoozeInput>(
                "anima_reminder_snooze",
                Access::Write,
                Effect::Sets,
                "Snoozes a reminder that was sent: the hub sends it again after `minutes`. \
                 CONFLICT when the firing was not sent or is decided. Call it only with the \
                 operator's decision.",
            )
            .returns(&[("firing", "object", "The snoozed firing.")]),
            Self::Ack => Definition::new::<AckInput>(
                "anima_reminder_ack",
                Access::Write,
                Effect::Sets,
                "Acknowledges a reminder that was sent, so that it is not sent again. Call it \
                 only with the operator's decision.",
            )
            .returns(&[("firing", "object", "The acknowledged firing.")]),
            _ => self.rule_definition(),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        let result = match self {
            Self::Test | Self::Firings | Self::Snooze | Self::Ack => {
                self.call_firing(hub, arguments).await?
            }
            _ => self.call_rule(hub, arguments).await?,
        };
        Ok(Value::Object(result))
    }

    /// Runs a tool that reads or changes rules.
    async fn call_rule(
        self,
        hub: &Hub,
        arguments: Option<JsonObject>,
    ) -> Result<serde_json::Map<String, Value>, ToolError> {
        Ok(match self {
            Self::List => {
                let NoInput {} = parse(arguments)?;
                let answer = hub.run::<ReminderList>(reminder_list::Variables).await?;
                object([("rules", cleaned(json(&answer.reminder_rules)))])
            }
            Self::Get => {
                let IdInput { id } = parse(arguments)?;
                let answer = hub
                    .run::<ReminderShow>(reminder_show::Variables { id })
                    .await?;
                object([("rule", cleaned(json(&answer.reminder_rule)))])
            }
            Self::Add => {
                let input = k1_input(rule_json(parse(arguments)?), "rule")?;
                let variables = reminder_add::Variables { input };
                let answer = hub.run::<ReminderAdd>(variables).await?;
                object([("rule", cleaned(json(&answer.create_reminder_rule)))])
            }
            Self::Set => {
                let SetInput { id, rule } = parse(arguments)?;
                let input = k1_input(rule_json(rule), "rule")?;
                let variables = reminder_set::Variables { id, input };
                let answer = hub.run::<ReminderSet>(variables).await?;
                object([("rule", cleaned(json(&answer.update_reminder_rule)))])
            }
            Self::Remove => {
                let IdInput { id } = parse(arguments)?;
                let variables = reminder_remove::Variables { id };
                let answer = hub.run::<ReminderRemove>(variables).await?;
                object([("removed", json(&answer.remove_reminder_rule))])
            }
            Self::SetEnabled => {
                let EnabledInput { id, enabled } = parse(arguments)?;
                let variables = reminder_enable::Variables { id, enabled };
                let answer = hub.run::<ReminderEnable>(variables).await?;
                object([("rule", cleaned(json(&answer.set_reminder_rule_enabled)))])
            }
            _ => {
                return Err(ToolError::Refused(
                    "This is not a tool of the rules.".to_owned(),
                ));
            }
        })
    }

    /// Runs a tool of the firings.
    async fn call_firing(
        self,
        hub: &Hub,
        arguments: Option<JsonObject>,
    ) -> Result<serde_json::Map<String, Value>, ToolError> {
        Ok(match self {
            Self::Test => {
                let IdInput { id } = parse(arguments)?;
                let answer = hub
                    .run::<ReminderTest>(reminder_test::Variables { id })
                    .await?;
                object([("firing", cleaned(json(&answer.test_reminder_rule)))])
            }
            Self::Firings => {
                let input: FiringsInput = parse(arguments)?;
                let variables = reminder_firings::Variables {
                    rule: input.rule,
                    state: input.state.as_deref().map(state).transpose()?,
                    first: input.first,
                };
                let answer = hub.run::<ReminderFirings>(variables).await?;
                object([("firings", cleaned(json(&answer.reminder_firings)))])
            }
            Self::Snooze => {
                let SnoozeInput { firing, minutes } = parse(arguments)?;
                let variables = reminder_snooze::Variables { firing, minutes };
                let answer = hub.run::<ReminderSnooze>(variables).await?;
                object([("firing", cleaned(json(&answer.snooze_reminder)))])
            }
            Self::Ack => {
                let AckInput { firing } = parse(arguments)?;
                let variables = reminder_ack::Variables { firing };
                let answer = hub.run::<ReminderAck>(variables).await?;
                object([("firing", cleaned(json(&answer.acknowledge_reminder)))])
            }
            _ => {
                return Err(ToolError::Refused(
                    "This is not a tool of the firings.".to_owned(),
                ));
            }
        })
    }
}
