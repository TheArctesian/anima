//! The evaluation of a rule, the instants of a time rule and the place state (REM-001,
//! REM-003, REM-004, REM-006; K-E6 Sections 2 to 4).

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use uuid::Uuid;

mod codec;
mod places;
mod rules;
mod stored;
mod swept;
mod times;

use super::*;
use crate::{
    engine::{
        locations::{Position, Provider, Sample},
        places::{Category, Centre, Place},
        records::Reference,
    },
    ids::{PlaceId, ReminderRuleId, SampleId},
};

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn rule(trigger: Trigger) -> Rule {
    Rule {
        id: ReminderRuleId::from(Uuid::from_u128(1)),
        name: "Train".to_owned(),
        message: "Time to train: {title}".to_owned(),
        trigger,
        unless: None,
        limits: Limits::default(),
        revision: 4,
        updated_at: at("2026-01-01T00:00:00Z"),
    }
}

fn moment<'a>(now: &str, new: &'a [Occurrence], known: &'a Known) -> Moment<'a> {
    Moment {
        now: at(now),
        new,
        places: &[],
        known,
    }
}

fn record(source: &str, kind: &str, id: &str, title: &str) -> Occurrence {
    Occurrence::Record {
        source: source.to_owned(),
        kind: kind.to_owned(),
        record: Reference::parse(&format!("{kind}:{id}")).unwrap(),
        title: title.to_owned(),
        class: Class::B,
        at: at("2026-11-16T12:00:00Z"),
    }
}

fn measure(key: &str, value: f64, when: &str) -> Occurrence {
    Occurrence::Measure {
        key: key.to_owned(),
        value,
        class: Class::B,
        at: at(when),
    }
}

fn daily(time: (u32, u32), days: Vec<Weekday>, zone: Tz) -> Trigger {
    Trigger::Time(TimeTrigger::Daily {
        at: NaiveTime::from_hms_opt(time.0, time.1, 0).unwrap(),
        days,
        zone,
    })
}

fn workout() -> Trigger {
    Trigger::SourceEvent {
        source: "strava".to_owned(),
        kind: "workout".to_owned(),
        contains: Some("RUN".to_owned()),
    }
}
