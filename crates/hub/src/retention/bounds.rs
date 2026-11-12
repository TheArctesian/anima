//! The range of days the operator may set for each subject, and its default (D-25, DB-011).
//! `migrations/0901_retention_recordings_removed.sql` states the same numbers in its check
//! constraint (it restates those of 0900 without `recordings`, R-113).

use super::model::Subject;
use crate::api_errors::FieldError;

/// The smallest, largest and default number of days a subject is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bounds {
    pub(super) min: u16,
    pub(super) max: u16,
    pub(super) default: u16,
}

pub(super) const fn bounds(subject: Subject) -> Bounds {
    let (min, max, default) = match subject {
        Subject::Heartbeats | Subject::Metrics => (1, 400, 30),
        Subject::Events => (30, 365, 90),
        Subject::Deliveries => (7, 365, 30),
    };
    Bounds { min, max, default }
}

/// `days` as a period of `subject`, or the error on the `days` field.
pub(super) fn checked(subject: Subject, days: i64) -> Result<u16, FieldError> {
    let Bounds { min, max, .. } = bounds(subject);
    u16::try_from(days)
        .ok()
        .filter(|days| (min..=max).contains(days))
        .ok_or_else(|| {
            FieldError::new(
                "days",
                format!("Keep {} for {min} to {max} days.", subject.as_str()),
            )
        })
}
