//! `anima reminder list|show|add|set|remove|enable|disable`. `--trigger` and `--unless` are
//! JSON in the form of K-E6 Section 2, for example
//! `{"type":"time","at":"09:00","zone":"Europe/Amsterdam"}`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::documents::{
    ReminderAdd, ReminderEnable, ReminderList, ReminderRemove, ReminderSet, ReminderShow,
    reminder_add, reminder_enable, reminder_list, reminder_remove, reminder_set, reminder_show,
};
use super::shown;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct IdArgs {
    /// The rule's ID (see `anima reminder list`).
    pub id: String,
}

/// The fields of a rule that `add` and `set` take.
#[derive(Debug, clap::Args)]
pub struct FieldArgs {
    /// The name (one line of 1 to 80 characters).
    #[arg(long)]
    name: Option<String>,
    /// The text of the reminder; {title} is the title of what it is about.
    #[arg(long)]
    message: Option<String>,
    /// The trigger, as JSON.
    #[arg(long)]
    trigger: Option<String>,
    /// The condition that stops the reminder, as JSON; on `set`, `null` removes it.
    #[arg(long)]
    unless: Option<String>,
    /// Minutes between two reminders of the rule, 1 to 10080.
    #[arg(long)]
    cooldown_minutes: Option<i64>,
    /// The most reminders of the rule in one day, 1 to 100.
    #[arg(long)]
    daily_limit: Option<i64>,
    /// A notification channel, by ID. Repeat for more; on `set` the list replaces the old one.
    #[arg(long = "channel")]
    channels: Vec<String>,
    /// On `set`: use each channel that is on by default.
    #[arg(long, conflicts_with = "channels")]
    default_channels: bool,
}

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    #[command(flatten)]
    fields: FieldArgs,
    /// Make the rule off.
    #[arg(long)]
    disabled: bool,
}

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// The rule's ID.
    id: String,
    #[command(flatten)]
    fields: FieldArgs,
}

/// The JSON of a flag. A refusal names the flag, never the text.
fn parsed(flag: &str, text: &str) -> Result<Value, Failure> {
    serde_json::from_str(text).map_err(|_| Failure::Usage {
        message: format!("{flag} is not JSON. Give it as in K-E6 Section 2, in single quotes."),
        hint: None,
    })
}

/// The K1 input that `json` holds. Each document has its own input type of the same shape.
fn k1<T: DeserializeOwned>(json: Value) -> Result<T, Failure> {
    serde_json::from_value(json).map_err(|_| Failure::Usage {
        message: "The fields of the rule do not make a rule.".to_owned(),
        hint: None,
    })
}

impl FieldArgs {
    /// `current` with each field that a flag gives.
    fn over(self, mut current: Value) -> Result<Value, Failure> {
        if let Some(name) = self.name {
            current["name"] = json!(name);
        }
        if let Some(message) = self.message {
            current["message"] = json!(message);
        }
        if let Some(trigger) = self.trigger {
            current["trigger"] = parsed("--trigger", &trigger)?;
        }
        if let Some(unless) = self.unless {
            current["unless"] = parsed("--unless", &unless)?;
        }
        if let Some(minutes) = self.cooldown_minutes {
            current["cooldownMinutes"] = json!(minutes);
        }
        if let Some(limit) = self.daily_limit {
            current["dailyLimit"] = json!(limit);
        }
        if !self.channels.is_empty() || self.default_channels {
            current["channelIds"] = json!(self.channels);
        }
        Ok(current)
    }
}

fn show_rule<T: Serialize>(invocation: &Invocation, rule: &T) -> Result<(), Failure> {
    invocation.output.data(rule, |rule| {
        shown::rule(&serde_json::to_value(rule).unwrap_or_default())
    })
}

/// `anima reminder list`.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn list(invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let answer = hub.run::<ReminderList>(reminder_list::Variables).await?;
    invocation.output.data(&answer.reminder_rules, |rules| {
        shown::rules(&serde_json::to_value(rules).unwrap_or_default())
    })
}

async fn fetch(
    invocation: &Invocation,
    id: &str,
) -> Result<reminder_show::ReminderShowReminderRule, Failure> {
    let hub = invocation.hub()?;
    let variables = reminder_show::Variables { id: id.to_owned() };
    let answer = hub.run::<ReminderShow>(variables).await?;
    answer.reminder_rule.ok_or_else(|| Failure::NotFound {
        message: "No reminder rule has this ID.".to_owned(),
        hint: None,
    })
}

/// `anima reminder show <id>`.
///
/// # Errors
/// [`Failure::NotFound`] when no rule has this ID; a hub refusal.
pub async fn show(invocation: &Invocation, id: &str) -> Result<(), Failure> {
    show_rule(invocation, &fetch(invocation, id).await?)
}

impl AddArgs {
    /// Makes the rule and prints it.
    ///
    /// # Errors
    /// [`Failure::Usage`] without a name, a message or a trigger, or for JSON that does not
    /// parse; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        for (given, what, flag) in [
            (self.fields.name.is_some(), "the name", "--name"),
            (self.fields.message.is_some(), "the message", "--message"),
            (self.fields.trigger.is_some(), "the trigger", "--trigger"),
        ] {
            if !given {
                return Err(Failure::missing(what, flag));
            }
        }
        let base = json!({ "enabled": !self.disabled, "cooldownMinutes": 60, "dailyLimit": 3 });
        let input = self.fields.over(base)?;
        let hub = invocation.hub()?;
        let variables = reminder_add::Variables { input: k1(input)? };
        let answer = hub.run::<ReminderAdd>(variables).await?;
        show_rule(invocation, &answer.create_reminder_rule)
    }
}

impl SetArgs {
    /// Reads the rule, changes what the flags give, and writes it back.
    ///
    /// # Errors
    /// [`Failure::NotFound`]; [`Failure::Usage`] for JSON that does not parse; the hub's
    /// refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let current = serde_json::to_value(fetch(invocation, &self.id).await?).unwrap_or_default();
        let mut input = json!({});
        for field in [
            "name",
            "message",
            "trigger",
            "unless",
            "cooldownMinutes",
            "dailyLimit",
            "channelIds",
            "enabled",
        ] {
            input[field] = current[field].clone();
        }
        let input = self.fields.over(input)?;
        let hub = invocation.hub()?;
        let variables = reminder_set::Variables {
            id: self.id,
            input: k1(input)?,
        };
        let answer = hub.run::<ReminderSet>(variables).await?;
        show_rule(invocation, &answer.update_reminder_rule)
    }
}

/// `anima reminder remove <id>`.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn remove(invocation: &Invocation, id: String) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let answer = hub
        .run::<ReminderRemove>(reminder_remove::Variables { id })
        .await?;
    if invocation.output.json() {
        return invocation
            .output
            .raw(&json!({ "removed": answer.remove_reminder_rule }));
    }
    invocation.output.line(&format!(
        "Deleted the reminder rule {} with its firings.",
        answer.remove_reminder_rule
    ))
}

/// `anima reminder enable <id>` and `disable <id>`.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn enable(invocation: &Invocation, id: String, enabled: bool) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let variables = reminder_enable::Variables { id, enabled };
    let answer = hub.run::<ReminderEnable>(variables).await?;
    show_rule(invocation, &answer.set_reminder_rule_enabled)
}
