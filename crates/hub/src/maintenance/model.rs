//! Domain types of maintenance windows (MW-001 to MW-004).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::ids::{CheckId, DeviceId, WindowId};

/// The five kinds of window the operator can create (spec 6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum WindowKind {
    Manual,
    Single,
    RecurringInterval,
    RecurringWeekday,
    RecurringDayOfMonth,
}

impl WindowKind {
    /// The `lower_snake` text stored in `windows.kind` and in the rule's `kind` tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Single => "single",
            Self::RecurringInterval => "recurring_interval",
            Self::RecurringWeekday => "recurring_weekday",
            Self::RecurringDayOfMonth => "recurring_day_of_month",
        }
    }
}

/// A day of the week; `Mon` is the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Enum)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl From<chrono::Weekday> for Weekday {
    fn from(day: chrono::Weekday) -> Self {
        match day {
            chrono::Weekday::Mon => Self::Mon,
            chrono::Weekday::Tue => Self::Tue,
            chrono::Weekday::Wed => Self::Wed,
            chrono::Weekday::Thu => Self::Thu,
            chrono::Weekday::Fri => Self::Fri,
            chrono::Weekday::Sat => Self::Sat,
            chrono::Weekday::Sun => Self::Sun,
        }
    }
}

/// When a window is active, as fields, never as cron text (MW-004). Local times are read in
/// the window's time zone (MW-001). It is stored as JSON in `windows.rule`.
///
/// A manual window's on/off state is not part of the rule: it lives in the window
/// (`Window::active_manual`), because the operator flips it without editing the rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum WindowRule {
    /// Active only while the operator has switched it on.
    Manual,
    /// Active from `starts_at` up to, not including, `ends_at`.
    Single {
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    },
    /// Starts at `first_start_at` and again at the same local time every `every_days` days.
    RecurringInterval {
        first_start_at: DateTime<Utc>,
        every_days: u32,
        duration_minutes: u32,
    },
    /// Starts at `start_time` local time on each listed weekday.
    RecurringWeekday {
        weekdays: Vec<Weekday>,
        start_time: NaiveTime,
        duration_minutes: u32,
    },
    /// Starts at `start_time` local time on each listed day of the month; `-1` is the last
    /// day, and a day the month does not have (31 in April) does not happen that month.
    RecurringDayOfMonth {
        days_of_month: Vec<i8>,
        start_time: NaiveTime,
        duration_minutes: u32,
    },
}

impl WindowRule {
    #[must_use]
    pub const fn kind(&self) -> WindowKind {
        match self {
            Self::Manual => WindowKind::Manual,
            Self::Single { .. } => WindowKind::Single,
            Self::RecurringInterval { .. } => WindowKind::RecurringInterval,
            Self::RecurringWeekday { .. } => WindowKind::RecurringWeekday,
            Self::RecurringDayOfMonth { .. } => WindowKind::RecurringDayOfMonth,
        }
    }

    /// How long each occurrence lasts; a single window has no repeating length.
    pub(super) const fn duration_minutes(&self) -> Option<u32> {
        match self {
            Self::RecurringInterval {
                duration_minutes, ..
            }
            | Self::RecurringWeekday {
                duration_minutes, ..
            }
            | Self::RecurringDayOfMonth {
                duration_minutes, ..
            } => Some(*duration_minutes),
            Self::Manual | Self::Single { .. } => None,
        }
    }
}

/// What a window applies to (MW-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum TargetType {
    Device,
    Check,
    Tag,
}

impl TargetType {
    /// The text stored in `window_targets.target_type`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Check => "check",
            Self::Tag => "tag",
        }
    }
}

/// One thing a window applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Device(DeviceId),
    Check(CheckId),
    Tag(String),
}

impl Target {
    #[must_use]
    pub const fn target_type(&self) -> TargetType {
        match self {
            Self::Device(_) => TargetType::Device,
            Self::Check(_) => TargetType::Check,
            Self::Tag(_) => TargetType::Tag,
        }
    }

    /// The device ID, check ID or tag as text, as stored in `window_targets.target`.
    #[must_use]
    pub fn value(&self) -> String {
        match self {
            Self::Device(id) => id.to_string(),
            Self::Check(id) => id.to_string(),
            Self::Tag(tag) => tag.clone(),
        }
    }
}

/// A stored maintenance window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub timezone: Tz,
    pub rule: WindowRule,
    /// The on/off state of a manual window; false for every other kind.
    pub active_manual: bool,
    pub targets: Vec<Target>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Window {
    /// Whether the window is in force at `now`. A manual window follows `active_manual`; the
    /// other kinds follow their rule.
    #[must_use]
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        match self.rule {
            WindowRule::Manual => self.active_manual,
            _ => super::schedule::is_active(&self.rule, self.timezone, now),
        }
    }
}

/// A validated window to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NewWindow {
    pub name: String,
    pub timezone: Tz,
    pub rule: WindowRule,
    pub targets: Vec<Target>,
}

/// One target as typed in the form.
#[derive(Clone, Debug)]
pub(super) struct TargetForm {
    pub kind: TargetType,
    pub target: String,
}

/// A window as typed in the form: every field of every kind is optional here, and the rules
/// decide which ones the kind needs.
#[derive(Clone, Debug)]
pub(super) struct WindowForm {
    pub name: String,
    pub kind: WindowKind,
    pub timezone: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub first_start_at: Option<DateTime<Utc>>,
    pub every_days: Option<i32>,
    pub weekdays: Option<Vec<Weekday>>,
    pub days_of_month: Option<Vec<i32>>,
    pub start_time: Option<String>,
    pub duration_minutes: Option<i32>,
    pub targets: Vec<TargetForm>,
}
