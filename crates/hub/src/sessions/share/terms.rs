//! The bounds of a share (D-9, S-3): how long it lasts and how many viewers it admits, with
//! K1's defaults. Pure.

use chrono::TimeDelta;

use super::model::ShareAccess;
use crate::sessions::FieldError;

/// `ShareInput.expiresInMinutes`: the fewest and most minutes a share lasts.
pub(super) const MINUTES: (i32, i32) = (5, 1440);

/// K1's default for `ShareInput.expiresInMinutes`.
pub(super) const DEFAULT_MINUTES: i32 = 60;

/// `ShareInput.maxViewers`: the fewest and most viewers a share admits.
pub(super) const VIEWERS: (i32, i32) = (1, 10);

/// K1's default for `ShareInput.maxViewers`.
pub(super) const DEFAULT_VIEWERS: i32 = 3;

/// A share the owner asked for, within the bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Terms {
    pub(super) access: ShareAccess,
    pub(super) lifetime: TimeDelta,
    pub(super) max_viewers: u8,
}

/// `value` as a `T` when it is within `min..=max`, else the field's error.
fn within<T: TryFrom<i32>>(
    value: i32,
    (min, max): (i32, i32),
    field: &'static str,
) -> Result<T, FieldError> {
    (min..=max)
        .contains(&value)
        .then(|| T::try_from(value).ok())
        .flatten()
        .ok_or_else(|| FieldError::new(field, format!("Enter a value from {min} to {max}.")))
}

/// The terms of a share with `access` lasting `minutes` for at most `viewers` viewers, else an
/// error for each field outside its bounds, in input order.
pub(super) fn checked_terms(
    access: ShareAccess,
    minutes: i32,
    viewers: i32,
) -> Result<Terms, Vec<FieldError>> {
    let minutes = within::<i64>(minutes, MINUTES, "expiresInMinutes");
    let viewers = within::<u8>(viewers, VIEWERS, "maxViewers");
    match (minutes, viewers) {
        (Ok(minutes), Ok(max_viewers)) => Ok(Terms {
            access,
            lifetime: TimeDelta::minutes(minutes),
            max_viewers,
        }),
        (minutes, viewers) => Err([minutes.err(), viewers.err()]
            .into_iter()
            .flatten()
            .collect()),
    }
}
