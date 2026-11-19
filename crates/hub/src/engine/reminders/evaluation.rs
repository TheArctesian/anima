//! The evaluation of one rule (K-E6 Section 3; REM-001, REM-003, REM-004). Pure: no clock, no
//! database and no random value. The order of the checks is fixed: the trigger is due; `unless`
//! is not met; the cooldown passed; the daily limit is not reached.

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use chrono_tz::Tz;

use crate::engine::records::Reference;

use super::{
    model::{
        Class, Crossing, Due, History, Moment, Occurrence, Rule, Stop, TimeTrigger, Trigger, Unless,
    },
    schedule,
};

/// The source of the occurrences that the source framework gives for a change of health.
const SOURCES: &str = "sources";
/// The oldest sample that still fires a place rule (R-145).
const PLACE_MAX_AGE: TimeDelta = TimeDelta::minutes(10);

/// A trigger that is due: what it is about, the title for the text, and its class.
struct Hit {
    subject: String,
    title: String,
    class: Class,
}

/// The start of the day of the rule at `now`: the local day of the zone of a time rule, else
/// the day of UTC (REM-004). The caller counts the firings of the day from it.
#[must_use]
pub fn day_start(rule: &Rule, now: DateTime<Utc>) -> DateTime<Utc> {
    let zone = match &rule.trigger {
        Trigger::Time(TimeTrigger::Daily { zone, .. }) => *zone,
        _ => Tz::UTC,
    };
    schedule::day_start(zone, now)
}

fn timed(rule: &Rule, trigger: &TimeTrigger, now: DateTime<Utc>) -> Vec<Hit> {
    schedule::latest(trigger, now)
        .filter(|at| *at > rule.updated_at)
        .map(|at| Hit {
            subject: format!("time:{}", at.to_rfc3339_opts(SecondsFormat::Secs, true)),
            title: rule.name.clone(),
            class: Class::C,
        })
        .into_iter()
        .collect()
}

fn placed(rule: &Rule, moment: &Moment<'_>) -> Vec<Hit> {
    let Trigger::Place { place, on } = &rule.trigger else {
        return Vec::new();
    };
    moment
        .places
        .iter()
        .filter(|change| change.place == *place && change.inside == (*on == Crossing::Arrive))
        .filter(|change| moment.now - change.observed_at <= PLACE_MAX_AGE)
        .map(|change| Hit {
            subject: format!("place:{}", change.sample),
            title: rule.name.clone(),
            // Where the operator is, is Class B data (R-146).
            class: Class::B,
        })
        .collect()
}

/// The hit of a record for a source event rule: the source and the kind are equal, and the
/// title contains `contains` in any case.
fn event_hit(
    (source, kind, contains): (&str, &str, Option<&str>),
    occurrence: &Occurrence,
) -> Option<Hit> {
    let Occurrence::Record {
        source: from,
        kind: of,
        record,
        title,
        class,
        ..
    } = occurrence
    else {
        return None;
    };
    let wanted = contains.unwrap_or_default().to_lowercase();
    (source == from && kind == of && title.to_lowercase().contains(&wanted)).then(|| Hit {
        subject: record.to_string(),
        title: title.clone(),
        class: *class,
    })
}

/// The hit of a change of health that the source framework gave, for the account of the rule
/// or for any account.
fn state_hit((account, state): (Option<&Reference>, &str), occurrence: &Occurrence) -> Option<Hit> {
    let Occurrence::Record {
        source,
        kind,
        record,
        title,
        class,
        at,
    } = occurrence
    else {
        return None;
    };
    let named = account.is_none_or(|account| account == record);
    (source == SOURCES && kind == state && named).then(|| Hit {
        subject: format!("{record}:{state}:{}", at.timestamp()),
        title: title.clone(),
        class: *class,
    })
}

/// The hit of an expiry: the nearest number of days that the time left is not more than. The
/// larger numbers do not fire for a record that is first seen late.
fn expiry_hit(
    (subject, days_before): (&str, &[u32]),
    occurrence: &Occurrence,
    now: DateTime<Utc>,
) -> Option<Hit> {
    let Occurrence::Expiry {
        subject: of,
        record,
        title,
        class,
        expires_at,
    } = occurrence
    else {
        return None;
    };
    let left = *expires_at - now;
    let nearest = days_before
        .iter()
        .filter(|days| left <= TimeDelta::days(i64::from(**days)))
        .min()?;
    (subject == of).then(|| Hit {
        subject: format!("{record}:{nearest}"),
        title: title.clone(),
        class: *class,
    })
}

/// The hit of one new occurrence for the trigger, when the trigger is about it.
fn occurred(trigger: &Trigger, occurrence: &Occurrence, now: DateTime<Utc>) -> Option<Hit> {
    match trigger {
        Trigger::SourceEvent {
            source,
            kind,
            contains,
        } => event_hit((source, kind, contains.as_deref()), occurrence),
        Trigger::SourceState { account, state } => state_hit((account.as_ref(), state), occurrence),
        Trigger::Expiry {
            subject,
            days_before,
        } => expiry_hit((subject, days_before), occurrence, now),
        _ => None,
    }
}

/// The hits of a threshold: each new measure of the key that is on the side of the value when
/// the measure before it was not. The measure before the first new one is the known one.
fn crossed(rule: &Rule, moment: &Moment<'_>) -> Vec<Hit> {
    let Trigger::Threshold { key, is, value } = &rule.trigger else {
        return Vec::new();
    };
    let mut before = moment.known.measures.get(key).copied();
    let mut hits = Vec::new();
    for occurrence in moment.new {
        let Occurrence::Measure {
            key: of,
            value: measured,
            class,
            at,
        } = occurrence
        else {
            continue;
        };
        if of != key {
            continue;
        }
        let was = before.is_some_and(|before| is.holds(before, *value));
        if is.holds(*measured, *value) && !was {
            hits.push(Hit {
                subject: format!("measure:{key}:{}", at.timestamp()),
                title: rule.name.clone(),
                class: *class,
            });
        }
        before = Some(*measured);
    }
    hits
}

fn hits(rule: &Rule, moment: &Moment<'_>) -> Vec<Hit> {
    match &rule.trigger {
        Trigger::Time(trigger) => timed(rule, trigger, moment.now),
        Trigger::Place { .. } => placed(rule, moment),
        Trigger::Threshold { .. } => crossed(rule, moment),
        trigger => moment
            .new
            .iter()
            .filter_map(|occurrence| occurred(trigger, occurrence, moment.now))
            .collect(),
    }
}

/// Whether the `unless` of the rule is met: by what was known, or by a new occurrence.
fn excused(unless: &Unless, moment: &Moment<'_>) -> bool {
    match unless {
        Unless::RecordToday { source, kind } => {
            let known = &moment.known.records_today;
            known.contains(&(source.clone(), kind.clone()))
                || moment.new.iter().any(|occurrence| {
                    matches!(occurrence, Occurrence::Record { source: from, kind: of, .. }
                        if from == source && of == kind)
                })
        }
        Unless::Measure { key, is, value } => {
            let newest = moment
                .new
                .iter()
                .rev()
                .find_map(|occurrence| match occurrence {
                    Occurrence::Measure {
                        key: of,
                        value: measured,
                        ..
                    } if of == key => Some(*measured),
                    _ => None,
                });
            newest
                .or_else(|| moment.known.measures.get(key).copied())
                .is_some_and(|measured| is.holds(measured, *value))
        }
    }
}

/// The firings of `rule` at `moment`: each one to send, and each one that the cooldown or the
/// daily limit stops. A subject that fired before gives nothing, and a rule whose `unless` is
/// met gives nothing.
#[must_use]
pub fn evaluate(rule: &Rule, history: &History, moment: &Moment<'_>) -> Vec<Due> {
    if rule
        .unless
        .as_ref()
        .is_some_and(|unless| excused(unless, moment))
    {
        return Vec::new();
    }
    let cooldown = TimeDelta::minutes(i64::from(rule.limits.cooldown_minutes));
    let (mut last_fired, mut fired_today) = (history.last_fired, history.fired_today);
    let mut seen: Vec<String> = Vec::new();
    let mut due = Vec::new();
    for hit in hits(rule, moment) {
        if history.subjects.contains(&hit.subject) || seen.contains(&hit.subject) {
            continue;
        }
        let stopped = if last_fired.is_some_and(|last| moment.now - last < cooldown) {
            Some(Stop::Cooldown)
        } else if fired_today >= rule.limits.daily_limit {
            Some(Stop::DailyLimit)
        } else {
            None
        };
        if stopped.is_none() {
            last_fired = Some(moment.now);
            fired_today += 1;
        }
        seen.push(hit.subject.clone());
        due.push(Due {
            rule: rule.id,
            revision: rule.revision,
            subject: hit.subject,
            text: rule.message.replace("{title}", &hit.title),
            class: hit.class,
            stopped,
        });
    }
    due
}
