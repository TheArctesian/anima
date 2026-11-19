//! What the operator does with rules and firings (K-E6 Sections 2, 5 and 6; REM-004, REM-005,
//! REM-007). Each change of a rule adds 1 to its revision, so a firing that was due before the
//! change is not sent.

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Error,
    codec::Fault,
    model::{Class, Due, Firing, FiringState, Limits, Rule, Trigger, Unless},
    store::{self, StoredRule},
};
use crate::ids::{ChannelId, FiringId, ReminderRuleId};

const NAME_MAX: usize = 80;
const MESSAGE_MAX: usize = 200;
/// The longest snooze and the longest cooldown: one week of minutes.
pub const MINUTES_MAX: u32 = 10_080;
const LIMIT_MAX: u32 = 100;
/// The most channels one rule names.
const CHANNELS_MAX: usize = 20;
/// The most firings one read gives.
pub const FIRINGS_MAX: i64 = 200;

/// A rule as the operator gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub name: String,
    pub message: String,
    pub trigger: Trigger,
    pub unless: Option<Unless>,
    pub limits: Limits,
    pub channels: Vec<ChannelId>,
    pub enabled: bool,
}

fn one_line(text: &str, most: usize) -> Option<String> {
    let text = text.trim();
    let valid =
        !text.is_empty() && text.chars().count() <= most && !text.chars().any(char::is_control);
    valid.then(|| text.to_owned())
}

/// The draft with its texts cleaned, or each field to fix.
fn checked(draft: Draft) -> Result<Draft, Vec<Fault>> {
    let fault = |field, message| Fault { field, message };
    let mut faults = Vec::new();
    let name = one_line(&draft.name, NAME_MAX);
    if name.is_none() {
        faults.push(fault("name", "Give one line of 1 to 80 characters."));
    }
    let message = one_line(&draft.message, MESSAGE_MAX);
    if message.is_none() {
        faults.push(fault("message", "Give one line of 1 to 200 characters."));
    }
    if !(1..=MINUTES_MAX).contains(&draft.limits.cooldown_minutes) {
        faults.push(fault("cooldownMinutes", "Give a number from 1 to 10080."));
    }
    if !(1..=LIMIT_MAX).contains(&draft.limits.daily_limit) {
        faults.push(fault("dailyLimit", "Give a number from 1 to 100."));
    }
    if draft.channels.len() > CHANNELS_MAX {
        faults.push(fault("channels", "Give at most 20 channels."));
    }
    match (name, message) {
        (Some(name), Some(message)) if faults.is_empty() => Ok(Draft {
            name,
            message,
            ..draft
        }),
        _ => Err(faults),
    }
}

fn stored(id: ReminderRuleId, draft: Draft, revision: i32, at: DateTime<Utc>) -> StoredRule {
    StoredRule {
        rule: Rule {
            id,
            name: draft.name,
            message: draft.message,
            trigger: draft.trigger,
            unless: draft.unless,
            limits: draft.limits,
            revision,
            updated_at: at,
        },
        channels: draft.channels,
        enabled: draft.enabled,
        created_at: at,
    }
}

/// Makes a rule in the caller's transaction.
///
/// # Errors
/// `Invalid` with each field to fix; `Database`.
pub async fn create_rule(
    conn: &mut PgConnection,
    draft: Draft,
    now: DateTime<Utc>,
) -> Result<StoredRule, Error> {
    let draft = checked(draft).map_err(Error::Invalid)?;
    let rule = stored(Uuid::now_v7().into(), draft, 1, now);
    store::insert_rule(conn, &rule).await?;
    Ok(rule)
}

/// Replaces the fields of the rule `id` with `draft` and adds 1 to its revision.
///
/// # Errors
/// `NotFound`; `Invalid` with each field to fix; `Database`.
pub async fn update_rule(
    conn: &mut PgConnection,
    id: ReminderRuleId,
    draft: Draft,
    now: DateTime<Utc>,
) -> Result<StoredRule, Error> {
    let draft = checked(draft).map_err(Error::Invalid)?;
    let before = store::rule(&mut *conn, id, true)
        .await?
        .ok_or(Error::NotFound("reminder rule"))?;
    let rule = StoredRule {
        created_at: before.created_at,
        ..stored(id, draft, before.rule.revision.saturating_add(1), now)
    };
    store::write_rule(conn, &rule).await?;
    Ok(rule)
}

/// Sets the rule `id` on or off. A change adds 1 to its revision; no change writes nothing.
/// Gives the rule and whether it changed.
///
/// # Errors
/// `NotFound`, `Database`.
pub async fn set_enabled(
    conn: &mut PgConnection,
    id: ReminderRuleId,
    enabled: bool,
    now: DateTime<Utc>,
) -> Result<(StoredRule, bool), Error> {
    let mut rule = store::rule(&mut *conn, id, true)
        .await?
        .ok_or(Error::NotFound("reminder rule"))?;
    if rule.enabled == enabled {
        return Ok((rule, false));
    }
    rule.enabled = enabled;
    rule.rule.revision = rule.rule.revision.saturating_add(1);
    rule.rule.updated_at = now;
    store::write_rule(conn, &rule).await?;
    Ok((rule, true))
}

/// Deletes the rule `id` with its firings, and gives the rule as it was.
///
/// # Errors
/// `NotFound`, `Database`.
pub async fn remove_rule(conn: &mut PgConnection, id: ReminderRuleId) -> Result<StoredRule, Error> {
    let rule = store::rule(&mut *conn, id, true)
        .await?
        .ok_or(Error::NotFound("reminder rule"))?;
    store::delete_rule(conn, id).await?;
    Ok(rule)
}

/// Every rule, by name.
///
/// # Errors
/// `Database`.
pub async fn rules(db: &PgPool) -> Result<Vec<StoredRule>, Error> {
    Ok(store::rules(db).await?)
}

/// The rule `id`.
///
/// # Errors
/// `Database`.
pub async fn rule(db: &PgPool, id: ReminderRuleId) -> Result<Option<StoredRule>, Error> {
    Ok(store::rule(db, id, false).await?)
}

/// For each rule that fired: the time of its newest firing, and how many of its firings since
/// `since` were not suppressed.
///
/// # Errors
/// `Database`.
pub async fn activity(
    db: &PgPool,
    since: DateTime<Utc>,
) -> Result<Vec<(ReminderRuleId, DateTime<Utc>, i64)>, Error> {
    Ok(store::activity(db, since).await?)
}

/// The firings of `rule` (or of every rule) in `state` (or any), newest first, at most `first`
/// (1 to [`FIRINGS_MAX`]).
///
/// # Errors
/// `Invalid` for a `first` out of range; `Database`.
pub async fn firings(
    db: &PgPool,
    filter: (Option<ReminderRuleId>, Option<FiringState>),
    first: i64,
) -> Result<Vec<Firing>, Error> {
    if !(1..=FIRINGS_MAX).contains(&first) {
        return Err(Error::Invalid(vec![Fault {
            field: "first",
            message: "Give a number from 1 to 200.",
        }]));
    }
    Ok(store::firings(db, filter, first).await?)
}

/// Makes one firing of the rule `id` that the sender sends at its next pass, with the subject
/// `test:<time>`. No cooldown and no limit apply, and none is used up: the history of a rule
/// does not count a test.
///
/// # Errors
/// `NotFound`, `Database`.
pub async fn test_rule(
    conn: &mut PgConnection,
    id: ReminderRuleId,
    now: DateTime<Utc>,
) -> Result<Firing, Error> {
    let rule = store::rule(&mut *conn, id, false)
        .await?
        .ok_or(Error::NotFound("reminder rule"))?
        .rule;
    let due = Due {
        rule: rule.id,
        revision: rule.revision,
        subject: format!("test:{}", now.to_rfc3339_opts(SecondsFormat::Micros, true)),
        text: rule.message.replace("{title}", &rule.name),
        class: Class::C,
        stopped: None,
    };
    store::insert_firing(conn, &due, now).await?;
    let made = store::firings(
        &mut *conn,
        (Some(id), Some(FiringState::Pending)),
        FIRINGS_MAX,
    )
    .await?
    .into_iter()
    .find(|firing| firing.subject == due.subject);
    made.ok_or(Error::NotFound("reminder firing"))
}

/// Writes the answer of the operator to a firing that was sent, and gives the firing.
async fn answered(
    conn: &mut PgConnection,
    id: FiringId,
    (state, snoozed_until): (FiringState, Option<DateTime<Utc>>),
    now: DateTime<Utc>,
) -> Result<Firing, Error> {
    let firing = store::firing(conn, id)
        .await?
        .ok_or(Error::NotFound("reminder firing"))?;
    if !matches!(firing.state, FiringState::Sent | FiringState::Snoozed) {
        return Err(Error::NotSent);
    }
    store::answer(conn, id, (state, snoozed_until), now).await?;
    Ok(Firing {
        state,
        snoozed_until,
        decided_at: Some(now),
        ..firing
    })
}

/// Snoozes a firing that was sent: the sender sends it again after `minutes` (1 to 10 080),
/// with the check at send time (REM-007).
///
/// # Errors
/// `Invalid` for the minutes; `NotFound`; `NotSent` for a firing that was not sent or is
/// decided; `Database`.
pub async fn snooze(
    conn: &mut PgConnection,
    id: FiringId,
    minutes: u32,
    now: DateTime<Utc>,
) -> Result<Firing, Error> {
    if !(1..=MINUTES_MAX).contains(&minutes) {
        return Err(Error::Invalid(vec![Fault {
            field: "minutes",
            message: "Give a number from 1 to 10080.",
        }]));
    }
    let until = now + TimeDelta::minutes(i64::from(minutes));
    answered(conn, id, (FiringState::Snoozed, Some(until)), now).await
}

/// Acknowledges a firing that was sent: it is not sent again (REM-007).
///
/// # Errors
/// `NotFound`; `NotSent`; `Database`.
pub async fn acknowledge(
    conn: &mut PgConnection,
    id: FiringId,
    now: DateTime<Utc>,
) -> Result<Firing, Error> {
    answered(conn, id, (FiringState::Acknowledged, None), now).await
}
