//! What happens when occurrences arrive, and the check at send time (K-E6 Section 3; REM-002,
//! REM-005). `notice` runs in the transaction that stores the new records.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgConnection;

use super::{
    Error,
    evaluation::{day_start, evaluate},
    model::{Class, Known, Moment, Occurrence, PlaceChange, Rule, Trigger},
    store,
};
use crate::ids::{ChannelId, FiringId, ReminderRuleId};

/// How far back the known records are read: more than one local day in any zone.
const KNOWN_BACK: TimeDelta = TimeDelta::hours(50);

/// The reason of a firing that is not sent because its rule changed, was set off or is gone.
pub const RULE_CHANGED: &str = "rule_changed";

/// What the hub knew before a moment, as the store keeps it: the newest time of each sort of
/// record and the newest measure of each key.
pub(super) struct WasKnown {
    records: Vec<(String, String, DateTime<Utc>)>,
    measures: HashMap<String, f64>,
}

impl WasKnown {
    pub(super) async fn read(conn: &mut PgConnection, now: DateTime<Utc>) -> Result<Self, Error> {
        let stored = store::known(conn, now - KNOWN_BACK).await?;
        let mut known = Self {
            records: Vec::new(),
            measures: HashMap::new(),
        };
        for (sort, name, value, at) in stored {
            match (sort.as_str(), name.split_once(':'), value) {
                ("measure", _, Some(value)) => {
                    known.measures.insert(name, value);
                }
                ("record", Some((source, kind)), _) => {
                    known.records.push((source.to_owned(), kind.to_owned(), at));
                }
                _ => {}
            }
        }
        Ok(known)
    }

    /// What `rule` can read: the records since the start of its day.
    fn since(&self, start: DateTime<Utc>) -> Known {
        let records_today: HashSet<(String, String)> = self
            .records
            .iter()
            .filter(|(_, _, at)| *at >= start)
            .map(|(source, kind, _)| (source.clone(), kind.clone()))
            .collect();
        Known {
            records_today,
            measures: self.measures.clone(),
        }
    }
}

/// Evaluates `rule` at `now` with the new occurrences and the place changes, and stores each
/// firing; gives how many were new.
pub(super) async fn fire(
    conn: &mut PgConnection,
    (rule, was_known): (&Rule, &WasKnown),
    (new, places): (&[Occurrence], &[PlaceChange]),
    now: DateTime<Utc>,
) -> Result<u64, Error> {
    let start = day_start(rule, now);
    let history = store::history(conn, rule.id, start).await?;
    let known = was_known.since(start);
    let moment = Moment {
        now,
        new,
        places,
        known: &known,
    };
    let mut stored = 0;
    for due in evaluate(rule, &history, &moment) {
        stored += u64::from(store::insert_firing(conn, &due, now).await?);
    }
    Ok(stored)
}

/// Evaluates the rules that wait for occurrences, in the transaction of the caller, and stores
/// each firing: `pending`, or `suppressed` with its reason. Then it keeps the new records and
/// measures as known. The same occurrences again give no second firing.
///
/// # Errors
/// `Database`.
pub async fn notice(
    conn: &mut PgConnection,
    new: &[Occurrence],
    now: DateTime<Utc>,
) -> Result<(), Error> {
    if new.is_empty() {
        return Ok(());
    }
    let known = WasKnown::read(conn, now).await?;
    for stored_rule in store::enabled_rules(conn).await? {
        let rule = stored_rule.rule;
        if !matches!(rule.trigger, Trigger::Time(_) | Trigger::Place { .. }) {
            fire(conn, (&rule, &known), (new, &[]), now).await?;
        }
    }
    for occurrence in new {
        match occurrence {
            Occurrence::Record {
                source, kind, at, ..
            } => {
                let name = format!("{source}:{kind}");
                store::remember(conn, ("record", &name), None, *at).await?;
            }
            Occurrence::Measure { key, value, at, .. } => {
                store::remember(conn, ("measure", key), Some(*value), *at).await?;
            }
            Occurrence::Expiry { .. } => {}
        }
    }
    Ok(())
}

/// A firing that passed the check at send time: what the delivery sends, and to which
/// channels of the rule (none means each channel that is on by default).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sendable {
    pub firing: FiringId,
    pub rule: ReminderRuleId,
    pub rule_name: String,
    pub text: String,
    pub class: Class,
    pub channels: Vec<ChannelId>,
}

/// The firings that wait for a send at `now`, oldest first, at most `limit`.
///
/// # Errors
/// `Database`.
pub async fn waiting(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<FiringId>, Error> {
    Ok(store::waiting(conn, now, limit).await?)
}

/// The check at send time, in the transaction that sends (REM-005): reads the rule again and
/// marks the firing `sent` only when the rule is enabled and has the revision of the firing.
/// Else the firing becomes `suppressed` with the reason `rule_changed` and nothing is sent.
/// `None` also for a firing that does not wait: one that is gone with its rule, was sent, or
/// is snoozed until a later time.
///
/// # Errors
/// `Database`.
pub async fn claim_send(
    conn: &mut PgConnection,
    firing: FiringId,
    now: DateTime<Utc>,
) -> Result<Option<Sendable>, Error> {
    let Some(row) = store::firing_for_send(conn, firing).await? else {
        return Ok(None);
    };
    let waits = match row.state.as_str() {
        "pending" => true,
        "snoozed" => row.snoozed_until.is_some_and(|until| until <= now),
        _ => false,
    };
    if !waits {
        return Ok(None);
    }
    if !row.rule_enabled || row.rule_revision != row.revision {
        store::decide(conn, row.id, ("suppressed", Some(RULE_CHANGED)), now).await?;
        return Ok(None);
    }
    store::decide(conn, row.id, ("sent", None), now).await?;
    Ok(Some(Sendable {
        firing: row.id,
        rule: row.rule_id,
        rule_name: row.rule_name,
        text: row.text,
        class: match row.class.as_str() {
            "A" => Class::A,
            "B" => Class::B,
            _ => Class::C,
        },
        channels: row.channels.into_iter().map(Into::into).collect(),
    }))
}
