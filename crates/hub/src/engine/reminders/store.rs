//! All SQL on schema `reminders`: I/O only.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

mod rules;
mod sweeping;

pub(super) use rules::{activity, answer, delete_rule, firing, firings, rule, rules, write_rule};
pub(super) use sweeping::{cursor, lock_sweep, place_states, put_place_state, set_cursor};

use super::{
    codec,
    model::{Class, Due, History, Limits, Rule},
};
use crate::ids::{ChannelId, FiringId, ReminderRuleId};

/// One rule with what the evaluation does not read.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredRule {
    pub rule: Rule,
    pub channels: Vec<ChannelId>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: ReminderRuleId,
    name: String,
    message: String,
    trigger: Json<Value>,
    unless: Option<Json<Value>>,
    cooldown_minutes: i32,
    daily_limit: i32,
    channels: Vec<Uuid>,
    enabled: bool,
    revision: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const RULE: &str = "id, name, message, trigger, unless, cooldown_minutes, daily_limit, channels, \
                    enabled, revision, created_at, updated_at";

impl TryFrom<RuleRow> for StoredRule {
    type Error = sqlx::Error;

    fn try_from(row: RuleRow) -> Result<Self, Self::Error> {
        let unreadable = |column: &str| sqlx::Error::Decode(format!("bad rule {column}").into());
        let unless = row
            .unless
            .map(|json| codec::unless(&json.0))
            .transpose()
            .map_err(|_| unreadable("unless"))?;
        Ok(Self {
            rule: Rule {
                id: row.id,
                name: row.name,
                message: row.message,
                trigger: codec::trigger(&row.trigger.0).map_err(|_| unreadable("trigger"))?,
                unless,
                limits: Limits {
                    cooldown_minutes: u32::try_from(row.cooldown_minutes)
                        .map_err(|_| unreadable("cooldown"))?,
                    daily_limit: u32::try_from(row.daily_limit).map_err(|_| unreadable("limit"))?,
                },
                revision: row.revision,
                updated_at: row.updated_at,
            },
            channels: row.channels.into_iter().map(Into::into).collect(),
            enabled: row.enabled,
            created_at: row.created_at,
        })
    }
}

pub(super) async fn insert_rule(
    conn: &mut PgConnection,
    stored: &StoredRule,
) -> Result<(), sqlx::Error> {
    let rule = &stored.rule;
    let channels: Vec<Uuid> = stored.channels.iter().map(|id| Uuid::from(*id)).collect();
    sqlx::query(
        "INSERT INTO reminders.rule (id, name, message, trigger, unless, cooldown_minutes, \
         daily_limit, channels, enabled, revision, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(rule.id)
    .bind(&rule.name)
    .bind(&rule.message)
    .bind(Json(codec::trigger_json(&rule.trigger)))
    .bind(
        rule.unless
            .as_ref()
            .map(|unless| Json(codec::unless_json(unless))),
    )
    .bind(i32::try_from(rule.limits.cooldown_minutes).unwrap_or(i32::MAX))
    .bind(i32::try_from(rule.limits.daily_limit).unwrap_or(i32::MAX))
    .bind(channels)
    .bind(stored.enabled)
    .bind(rule.revision)
    .bind(stored.created_at)
    .bind(rule.updated_at)
    .execute(conn)
    .await
    .map(drop)
}

/// Each rule that is enabled, oldest first.
pub(super) async fn enabled_rules(conn: &mut PgConnection) -> Result<Vec<StoredRule>, sqlx::Error> {
    let rows: Vec<RuleRow> = sqlx::query_as(&format!(
        "SELECT {RULE} FROM reminders.rule WHERE enabled ORDER BY created_at, id"
    ))
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(TryInto::try_into).collect()
}

/// What `rule` sent before: the last firing that was not suppressed, the count of them since
/// `day_start`, and each subject that fired.
pub(super) async fn history(
    conn: &mut PgConnection,
    rule: ReminderRuleId,
    day_start: DateTime<Utc>,
) -> Result<History, sqlx::Error> {
    let (last_fired, fired_today): (Option<DateTime<Utc>>, i64) = sqlx::query_as(
        "SELECT max(due_at), count(*) FILTER (WHERE due_at >= $2) FROM reminders.firing \
         WHERE rule_id = $1 AND state <> 'suppressed' AND subject NOT LIKE 'test:%'",
    )
    .bind(rule)
    .bind(day_start)
    .fetch_one(&mut *conn)
    .await?;
    let subjects: Vec<String> =
        sqlx::query_scalar("SELECT subject FROM reminders.firing WHERE rule_id = $1")
            .bind(rule)
            .fetch_all(conn)
            .await?;
    Ok(History {
        last_fired,
        fired_today: u32::try_from(fired_today).unwrap_or(u32::MAX),
        subjects: subjects.into_iter().collect::<HashSet<String>>(),
    })
}

/// Stores one firing: `pending`, or `suppressed` with its reason. `false` when the rule has a
/// firing for this subject, which then stays as it is.
pub(super) async fn insert_firing(
    conn: &mut PgConnection,
    due: &Due,
    at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let class = match due.class {
        Class::A => "A",
        Class::B => "B",
        Class::C => "C",
    };
    let state = if due.stopped.is_some() {
        "suppressed"
    } else {
        "pending"
    };
    let done = sqlx::query(
        "INSERT INTO reminders.firing (id, rule_id, revision, subject, text, class, state, \
         reason, due_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (rule_id, subject) DO NOTHING",
    )
    .bind(FiringId::from(Uuid::now_v7()))
    .bind(due.rule)
    .bind(due.revision)
    .bind(&due.subject)
    .bind(&due.text)
    .bind(class)
    .bind(state)
    .bind(due.stopped.map(super::model::Stop::as_str))
    .bind(at)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// The newest time of each sort of record since `from`, as `<source>:<kind>`, and the newest
/// measure of each key.
pub(super) async fn known(
    conn: &mut PgConnection,
    from: DateTime<Utc>,
) -> Result<Vec<(String, String, Option<f64>, DateTime<Utc>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT sort, name, value, at FROM reminders.known \
         WHERE sort = 'measure' OR at >= $1",
    )
    .bind(from)
    .fetch_all(conn)
    .await
}

/// Keeps `at` (and `value`) for `(sort, name)` when it is newer than what is stored.
pub(super) async fn remember(
    conn: &mut PgConnection,
    (sort, name): (&str, &str),
    value: Option<f64>,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO reminders.known (sort, name, value, at) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (sort, name) DO UPDATE SET value = excluded.value, at = excluded.at \
         WHERE reminders.known.at <= excluded.at",
    )
    .bind(sort)
    .bind(name)
    .bind(value)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

/// The class whose stored text is `text`.
fn class(text: &str) -> Option<Class> {
    match text {
        "A" => Some(Class::A),
        "B" => Some(Class::B),
        "C" => Some(Class::C),
        _ => None,
    }
}

/// A firing with the state of its rule, locked for the send.
#[derive(sqlx::FromRow)]
pub(super) struct FiringRow {
    pub id: FiringId,
    pub rule_id: ReminderRuleId,
    pub revision: i32,
    pub text: String,
    pub class: String,
    pub state: String,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub rule_name: String,
    pub rule_revision: i32,
    pub rule_enabled: bool,
    pub channels: Vec<Uuid>,
}

pub(super) async fn firing_for_send(
    conn: &mut PgConnection,
    id: FiringId,
) -> Result<Option<FiringRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT f.id, f.rule_id, f.revision, f.text, f.class, f.state, f.snoozed_until, \
         r.name AS rule_name, r.revision AS rule_revision, r.enabled AS rule_enabled, r.channels \
         FROM reminders.firing f JOIN reminders.rule r ON r.id = f.rule_id \
         WHERE f.id = $1 FOR UPDATE OF f, r",
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

/// Sets the state of a firing: `sent` with its time, or `suppressed` with its reason and no
/// time of a send.
pub(super) async fn decide(
    conn: &mut PgConnection,
    id: FiringId,
    (state, reason): (&str, Option<&str>),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE reminders.firing SET state = $2, reason = $3, snoozed_until = NULL, \
         sent_at = CASE WHEN $2 = 'sent' THEN $4 ELSE NULL END, \
         decided_at = CASE WHEN $2 = 'sent' THEN decided_at ELSE $4 END WHERE id = $1",
    )
    .bind(id)
    .bind(state)
    .bind(reason)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

/// The firings that wait for a send at `now`: pending, or snoozed until a time that passed.
pub(super) async fn waiting(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<FiringId>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM reminders.firing WHERE state = 'pending' \
         OR (state = 'snoozed' AND snoozed_until <= $1) ORDER BY due_at, id LIMIT $2",
    )
    .bind(now)
    .bind(limit)
    .fetch_all(conn)
    .await
}
