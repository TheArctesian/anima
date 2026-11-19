//! The SQL of the rules and of the firings that the operator reads and changes (K-E6
//! Section 6).

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, types::Json};
use uuid::Uuid;

use super::{RULE, RuleRow, StoredRule};
use crate::{
    engine::reminders::{
        codec,
        model::{Firing, FiringState},
    },
    ids::{FiringId, ReminderRuleId},
};

/// Every rule, by name.
pub(in crate::engine::reminders) async fn rules(
    db: impl PgExecutor<'_>,
) -> Result<Vec<StoredRule>, sqlx::Error> {
    let rows: Vec<RuleRow> = sqlx::query_as(&format!(
        "SELECT {RULE} FROM reminders.rule ORDER BY name, id"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(TryInto::try_into).collect()
}

/// The rule `id`; its row is locked when `lock` is set.
pub(in crate::engine::reminders) async fn rule(
    db: impl PgExecutor<'_>,
    id: ReminderRuleId,
    lock: bool,
) -> Result<Option<StoredRule>, sqlx::Error> {
    let tail = if lock { " FOR UPDATE" } else { "" };
    let row: Option<RuleRow> = sqlx::query_as(&format!(
        "SELECT {RULE} FROM reminders.rule WHERE id = $1{tail}"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(TryInto::try_into).transpose()
}

/// Writes each field of `stored` that the operator can change, with its revision and the time
/// of the change.
pub(in crate::engine::reminders) async fn write_rule(
    conn: &mut PgConnection,
    stored: &StoredRule,
) -> Result<(), sqlx::Error> {
    let rule = &stored.rule;
    let channels: Vec<Uuid> = stored.channels.iter().map(|id| Uuid::from(*id)).collect();
    sqlx::query(
        "UPDATE reminders.rule SET name = $2, message = $3, trigger = $4, unless = $5, \
         cooldown_minutes = $6, daily_limit = $7, channels = $8, enabled = $9, revision = $10, \
         updated_at = $11 WHERE id = $1",
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
    .bind(rule.updated_at)
    .execute(conn)
    .await
    .map(drop)
}

pub(in crate::engine::reminders) async fn delete_rule(
    conn: &mut PgConnection,
    id: ReminderRuleId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM reminders.rule WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map(drop)
}

#[derive(sqlx::FromRow)]
struct FiringRow {
    id: FiringId,
    rule_id: ReminderRuleId,
    revision: i32,
    subject: String,
    text: String,
    class: String,
    state: String,
    reason: Option<String>,
    due_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    snoozed_until: Option<DateTime<Utc>>,
    decided_at: Option<DateTime<Utc>>,
}

const FIRING: &str = "id, rule_id, revision, subject, text, class, state, reason, due_at, \
                      sent_at, snoozed_until, decided_at";

impl TryFrom<FiringRow> for Firing {
    type Error = sqlx::Error;

    fn try_from(row: FiringRow) -> Result<Self, Self::Error> {
        let unreadable = |column: &str| sqlx::Error::Decode(format!("bad firing {column}").into());
        Ok(Self {
            id: row.id,
            rule: row.rule_id,
            revision: row.revision,
            subject: row.subject,
            text: row.text,
            class: super::class(&row.class).ok_or_else(|| unreadable("class"))?,
            state: FiringState::parse(&row.state).ok_or_else(|| unreadable("state"))?,
            reason: row.reason,
            due_at: row.due_at,
            sent_at: row.sent_at,
            snoozed_until: row.snoozed_until,
            decided_at: row.decided_at,
        })
    }
}

/// The firings of `rule` (or of every rule) in `state` (or any), newest first, at most `limit`.
pub(in crate::engine::reminders) async fn firings(
    db: impl PgExecutor<'_>,
    (rule, state): (Option<ReminderRuleId>, Option<FiringState>),
    limit: i64,
) -> Result<Vec<Firing>, sqlx::Error> {
    let rows: Vec<FiringRow> = sqlx::query_as(&format!(
        "SELECT {FIRING} FROM reminders.firing \
         WHERE ($1::uuid IS NULL OR rule_id = $1) AND ($2::text IS NULL OR state = $2) \
         ORDER BY due_at DESC, id DESC LIMIT $3"
    ))
    .bind(rule)
    .bind(state.map(FiringState::as_str))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(TryInto::try_into).collect()
}

/// The firing `id`, locked.
pub(in crate::engine::reminders) async fn firing(
    conn: &mut PgConnection,
    id: FiringId,
) -> Result<Option<Firing>, sqlx::Error> {
    let row: Option<FiringRow> = sqlx::query_as(&format!(
        "SELECT {FIRING} FROM reminders.firing WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(TryInto::try_into).transpose()
}

/// Writes the decision of the operator about a firing: its state, the time of the decision
/// and, for a snooze, the time to send it again.
pub(in crate::engine::reminders) async fn answer(
    conn: &mut PgConnection,
    id: FiringId,
    (state, snoozed_until): (FiringState, Option<DateTime<Utc>>),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE reminders.firing SET state = $2, snoozed_until = $3, decided_at = $4 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(state.as_str())
    .bind(snoozed_until)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

/// The newest firing of each rule and the count of the firings of each rule since `since` that
/// were not suppressed, as `(rule, newest due time, count)`.
pub(in crate::engine::reminders) async fn activity(
    db: impl PgExecutor<'_>,
    since: DateTime<Utc>,
) -> Result<Vec<(ReminderRuleId, DateTime<Utc>, i64)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT rule_id, max(due_at), \
         count(*) FILTER (WHERE due_at >= $1 AND state <> 'suppressed') \
         FROM reminders.firing GROUP BY rule_id",
    )
    .bind(since)
    .fetch_all(db)
    .await
}
