//! The reminders GraphQL part (K1; K-E6 Section 6): the rules, their firings, a test, and the
//! answers of the operator to a firing (REM-007). The subscription is not built yet.
//!
//! Each change of a rule writes its row, one audit entry and one event in one transaction.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

mod views;

use std::sync::Arc;

use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions as _, ID, Object, Result, Value,
};
use chrono::{DateTime, Utc};

use views::{FiringStateView, FiringView, RuleInput, RuleView};

use super::{
    Draft, Error, FIRINGS_MAX, Fault, StoredRule, Trigger, acknowledge, activity, create_rule,
    firings, remove_rule, rule, rules, schedule, set_enabled, snooze, test_rule, trail,
    update_rule,
};
use crate::{
    api_errors::{ErrorCode, begin, coded, commit, internal, pool, schema_dependency},
    clock::Clock,
    engine::places,
    fleet::{access::Authenticated, events::Trail, notifications},
    ids::{FiringId, ReminderRuleId},
};

/// `VALIDATION` with each field to fix.
fn invalid_fields(faults: &[Fault]) -> GraphQlError {
    let listed: Vec<Value> = faults
        .iter()
        .map(|fault| async_graphql::value!({ "field": fault.field, "message": fault.message }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", ErrorCode::Validation.as_str());
        extensions.set("fields", Value::List(listed));
    })
}

/// The K7 code of each refusal; a fault of the hub is `INTERNAL` with no detail (API-010).
fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(faults) => invalid_fields(&faults),
        Error::NotFound(what) => coded(format!("No {what} has this ID."), ErrorCode::NotFound),
        Error::NotSent => coded(
            "The firing was not sent, or it is acknowledged or suppressed.",
            ErrorCode::Conflict,
        ),
        other => internal(other),
    }
}

fn rule_id(id: &ID) -> Result<ReminderRuleId> {
    id.parse()
        .map_err(|_| refused(Error::NotFound("reminder rule")))
}

fn firing_id(id: &ID) -> Result<FiringId> {
    id.parse()
        .map_err(|_| refused(Error::NotFound("reminder firing")))
}

/// The draft of `input`, with the place and the channels checked against the hub.
async fn draft(ctx: &Context<'_>, input: RuleInput) -> Result<Draft> {
    let draft = input.draft().map_err(|fault| invalid_fields(&[fault]))?;
    let db = pool(ctx)?;
    if let Trigger::Place { place, .. } = &draft.trigger {
        let found = places::current(db, *place).await.map_err(internal)?;
        if found.is_none_or(|found| found.id != *place) {
            let fault = Fault {
                field: "trigger.place",
                message: "Give a place that exists and is not merged.",
            };
            return Err(invalid_fields(&[fault]));
        }
    }
    let known = notifications::channels_exist(db, &draft.channels)
        .await
        .map_err(internal)?;
    if !known {
        let fault = Fault {
            field: "channelIds",
            message: "Give notification channels that exist.",
        };
        return Err(invalid_fields(&[fault]));
    }
    Ok(draft)
}

fn now(ctx: &Context<'_>) -> Result<DateTime<Utc>> {
    Ok(schema_dependency::<Arc<dyn Clock>>(ctx)?.now())
}

/// The rules as K1 shows them, with the time of the newest firing of each and its count of
/// the day of UTC.
async fn shown(ctx: &Context<'_>, stored: Vec<StoredRule>) -> Result<Vec<RuleView>> {
    let since = schedule::day_start(chrono_tz::Tz::UTC, now(ctx)?);
    let fired = activity(pool(ctx)?, since).await.map_err(refused)?;
    Ok(stored
        .into_iter()
        .map(|stored| {
            let id = stored.rule.id;
            let found = fired.iter().find(|(rule, ..)| *rule == id);
            RuleView::new(stored, found.map(|(_, last, count)| (*last, *count)))
        })
        .collect())
}

async fn show(ctx: &Context<'_>, stored: StoredRule) -> Result<RuleView> {
    let mut all = shown(ctx, vec![stored]).await?;
    all.pop()
        .ok_or_else(|| refused(Error::NotFound("reminder rule")))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "RemindersQuery")]
impl Query {
    /// The reminder rules, by name.
    #[graphql(guard = "Authenticated")]
    async fn reminder_rules(&self, ctx: &Context<'_>) -> Result<Vec<RuleView>> {
        let stored = rules(pool(ctx)?).await.map_err(refused)?;
        shown(ctx, stored).await
    }

    /// One reminder rule; null when no rule has this ID.
    #[graphql(guard = "Authenticated")]
    async fn reminder_rule(&self, ctx: &Context<'_>, id: ID) -> Result<Option<RuleView>> {
        let Ok(id) = id.parse::<ReminderRuleId>() else {
            return Ok(None);
        };
        match rule(pool(ctx)?, id).await.map_err(refused)? {
            Some(stored) => Ok(Some(show(ctx, stored).await?)),
            None => Ok(None),
        }
    }

    /// The firings of a rule (or of every rule) in a state (or any), newest first, at most
    /// `first` (1 to 200; 200 when not given).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.reminderFirings"
    )]
    async fn reminder_firings(
        &self,
        ctx: &Context<'_>,
        rule: Option<ID>,
        state: Option<FiringStateView>,
        first: Option<i32>,
    ) -> Result<Vec<FiringView>> {
        let rule = rule.as_ref().map(rule_id).transpose()?;
        let filter = (rule, state.map(Into::into));
        let first = first.map_or(FIRINGS_MAX, i64::from);
        let found = firings(pool(ctx)?, filter, first).await.map_err(refused)?;
        Ok(found.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "RemindersMutation")]
impl Mutation {
    /// Makes a reminder rule.
    #[graphql(guard = "Authenticated")]
    async fn create_reminder_rule(&self, ctx: &Context<'_>, input: RuleInput) -> Result<RuleView> {
        let draft = draft(ctx, input).await?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let made = create_rule(&mut tx, draft, change.now)
            .await
            .map_err(refused)?;
        let entry = trail::created(&made);
        change.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        show(ctx, made).await
    }

    /// Replaces a reminder rule. A firing that was due before the change is not sent.
    #[graphql(guard = "Authenticated")]
    async fn update_reminder_rule(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: RuleInput,
    ) -> Result<RuleView> {
        let id = rule_id(&id)?;
        let draft = draft(ctx, input).await?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let changed = update_rule(&mut tx, id, draft, change.now)
            .await
            .map_err(refused)?;
        let entry = trail::updated(&changed);
        change.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        show(ctx, changed).await
    }

    /// Deletes a reminder rule with its firings, and returns its ID.
    #[graphql(guard = "Authenticated")]
    async fn remove_reminder_rule(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let rule = rule_id(&id)?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let removed = remove_rule(&mut tx, rule).await.map_err(refused)?;
        let entry = trail::removed(&removed);
        change.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }

    /// Sets a reminder rule on or off. A rule that is off fires nothing, and a firing that
    /// waits is not sent.
    #[graphql(guard = "Authenticated")]
    async fn set_reminder_rule_enabled(
        &self,
        ctx: &Context<'_>,
        id: ID,
        enabled: bool,
    ) -> Result<RuleView> {
        let id = rule_id(&id)?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (set, changed) = set_enabled(&mut tx, id, enabled, change.now)
            .await
            .map_err(refused)?;
        if changed {
            let entry = trail::switched(&set);
            change.write(&mut tx, entry).await.map_err(internal)?;
        }
        commit(tx).await?;
        show(ctx, set).await
    }

    /// Makes one firing of the rule that the hub sends in the next minute. No cooldown and no
    /// limit apply, and none is used up.
    #[graphql(guard = "Authenticated")]
    async fn test_reminder_rule(&self, ctx: &Context<'_>, id: ID) -> Result<FiringView> {
        let id = rule_id(&id)?;
        let at = now(ctx)?;
        let mut tx = begin(ctx).await?;
        let firing = test_rule(&mut tx, id, at).await.map_err(refused)?;
        commit(tx).await?;
        Ok(firing.into())
    }

    /// Snoozes a firing that was sent: the hub sends it again after `minutes` (1 to 10080).
    #[graphql(guard = "Authenticated")]
    async fn snooze_reminder(
        &self,
        ctx: &Context<'_>,
        firing: ID,
        minutes: i32,
    ) -> Result<FiringView> {
        let id = firing_id(&firing)?;
        let minutes = u32::try_from(minutes).unwrap_or_default();
        let at = now(ctx)?;
        let mut tx = begin(ctx).await?;
        let snoozed = snooze(&mut tx, id, minutes, at).await.map_err(refused)?;
        commit(tx).await?;
        Ok(snoozed.into())
    }

    /// Acknowledges a firing that was sent: the hub does not send it again.
    #[graphql(guard = "Authenticated")]
    async fn acknowledge_reminder(&self, ctx: &Context<'_>, firing: ID) -> Result<FiringView> {
        let id = firing_id(&firing)?;
        let at = now(ctx)?;
        let mut tx = begin(ctx).await?;
        let done = acknowledge(&mut tx, id, at).await.map_err(refused)?;
        commit(tx).await?;
        Ok(done.into())
    }
}
