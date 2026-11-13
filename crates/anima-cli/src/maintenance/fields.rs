//! The flags of a maintenance window, one for each field of the K1 `MaintenanceWindowInput`
//! (kebab-case), and the JSON input they make. Create and update each deserialise that JSON
//! into their own generated input type, so serde checks every value.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::targets;
use crate::failure::{Failure, Hint};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum KindArg {
    Manual,
    Single,
    RecurringInterval,
    RecurringWeekday,
    RecurringDayOfMonth,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum WeekdayArg {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

#[derive(Debug, Default, clap::Args)]
pub struct WindowFields {
    /// The window's name.
    #[arg(long)]
    name: Option<String>,
    /// How the window is scheduled.
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    /// The time zone the times are local to, such as Europe/Berlin.
    #[arg(long)]
    timezone: Option<String>,
    /// `SINGLE`: when it starts (RFC 3339).
    #[arg(long)]
    starts_at: Option<String>,
    /// `SINGLE`: when it ends (RFC 3339).
    #[arg(long)]
    ends_at: Option<String>,
    /// `RECURRING_INTERVAL`: the first start (RFC 3339).
    #[arg(long)]
    first_start_at: Option<String>,
    /// `RECURRING_INTERVAL`: days between starts.
    #[arg(long)]
    every_days: Option<i64>,
    /// `RECURRING_WEEKDAY`: weekdays, such as mon,wed (or repeat the flag).
    #[arg(long, value_enum, value_delimiter = ',')]
    weekdays: Vec<WeekdayArg>,
    /// `RECURRING_DAY_OF_MONTH`: 1 to 31, or -1 for the last day, such as 1,15 (or repeat the flag).
    #[arg(long, allow_hyphen_values = true, value_delimiter = ',')]
    days_of_month: Vec<i64>,
    /// `RECURRING_WEEKDAY` and `RECURRING_DAY_OF_MONTH`: the local start, HH:MM.
    #[arg(long)]
    start_time: Option<String>,
    /// How long a recurring window lasts, in minutes.
    #[arg(long)]
    duration_minutes: Option<i64>,
    /// What the window covers: device:ID-or-name, check:ID or check:DEVICE/NAME, tag:NAME, comma separated (or repeat the flag).
    #[arg(long, value_name = "TYPE:TARGET", value_delimiter = ',')]
    targets: Vec<String>,
}

/// The input fields that belong to one kind of schedule only.
const SCHEDULE: [&str; 8] = [
    "startsAt",
    "endsAt",
    "firstStartAt",
    "everyDays",
    "weekdays",
    "daysOfMonth",
    "startTime",
    "durationMinutes",
];

/// The fields of `MaintenanceWindowInput`; the rest of a listed window is not input.
pub const INPUT: [&str; 12] = [
    "name",
    "kind",
    "timezone",
    "startsAt",
    "endsAt",
    "firstStartAt",
    "everyDays",
    "weekdays",
    "daysOfMonth",
    "startTime",
    "durationMinutes",
    "targets",
];

impl WindowFields {
    /// Whether no flag is given.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.kind.is_none()
            && self.timezone.is_none()
            && self.starts_at.is_none()
            && self.ends_at.is_none()
            && self.first_start_at.is_none()
            && self.every_days.is_none()
            && self.weekdays.is_empty()
            && self.days_of_month.is_empty()
            && self.start_time.is_none()
            && self.duration_minutes.is_none()
            && self.targets.is_empty()
    }

    /// `base` (a window's input fields, or none) with the flags laid over it. A new kind drops
    /// the old kind's schedule fields.
    ///
    /// # Errors
    /// [`Failure::Usage`] for a target that is not `TYPE:TARGET`.
    pub fn apply(&self, mut base: Map<String, Value>) -> Result<Value, Failure> {
        if let Some(kind) = self.kind {
            let changed = base.get("kind").is_some_and(|old| *old != json!(kind.k1()));
            if changed {
                base.retain(|key, _| !SCHEDULE.contains(&key.as_str()));
            }
            base.insert("kind".to_owned(), json!(kind.k1()));
        }
        let texts = [
            ("name", &self.name),
            ("timezone", &self.timezone),
            ("startsAt", &self.starts_at),
            ("endsAt", &self.ends_at),
            ("firstStartAt", &self.first_start_at),
            ("startTime", &self.start_time),
        ];
        for (key, value) in texts {
            if let Some(text) = value {
                base.insert(key.to_owned(), json!(text));
            }
        }
        let numbers = [
            ("everyDays", self.every_days),
            ("durationMinutes", self.duration_minutes),
        ];
        for (key, value) in numbers {
            if let Some(number) = value {
                base.insert(key.to_owned(), json!(number));
            }
        }
        if !self.weekdays.is_empty() {
            let days: Vec<&str> = self.weekdays.iter().map(|day| day.k1()).collect();
            base.insert("weekdays".to_owned(), json!(days));
        }
        if !self.days_of_month.is_empty() {
            base.insert("daysOfMonth".to_owned(), json!(self.days_of_month));
        }
        if !self.targets.is_empty() {
            let targets = self
                .targets
                .iter()
                .map(|text| targets::parse(text))
                .collect::<Result<Vec<_>, _>>()?;
            base.insert("targets".to_owned(), json!(targets));
        }
        Ok(Value::Object(base))
    }
}

/// Fails when `input` lacks a field every window needs.
///
/// # Errors
/// [`Failure::Usage`] naming the missing flag.
pub fn require(input: &Value) -> Result<(), Failure> {
    for needed in ["name", "kind", "timezone"] {
        if input.get(needed).is_none_or(Value::is_null) {
            return Err(Failure::Usage {
                message: format!("--{needed} is needed."),
                hint: Some(Hint::run("anima maintenance create --help")),
            });
        }
    }
    Ok(())
}

/// The input as one operation's generated type; serde checks every value.
///
/// # Errors
/// [`Failure::Usage`] naming the fields a new window needs, or the one that does not fit.
pub fn into_input<T: DeserializeOwned>(input: Value) -> Result<T, Failure> {
    require(&input)?;
    let mut input = input;
    if input.get("targets").is_none() {
        input["targets"] = json!([]);
    }
    serde_json::from_value(input).map_err(|error| Failure::Usage {
        message: format!("A maintenance flag has a value the hub cannot read: {error}."),
        hint: Some(Hint::run("anima maintenance create --help")),
    })
}

impl KindArg {
    const fn k1(self) -> &'static str {
        match self {
            Self::Manual => "MANUAL",
            Self::Single => "SINGLE",
            Self::RecurringInterval => "RECURRING_INTERVAL",
            Self::RecurringWeekday => "RECURRING_WEEKDAY",
            Self::RecurringDayOfMonth => "RECURRING_DAY_OF_MONTH",
        }
    }
}

impl WeekdayArg {
    const fn k1(self) -> &'static str {
        match self {
            Self::Mon => "MON",
            Self::Tue => "TUE",
            Self::Wed => "WED",
            Self::Thu => "THU",
            Self::Fri => "FRI",
            Self::Sat => "SAT",
            Self::Sun => "SUN",
        }
    }
}
