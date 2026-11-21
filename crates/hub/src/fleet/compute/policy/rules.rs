//! The device policy of a compute node (CAP-010 to CAP-012): the limits and the times in which
//! the node takes model work. Pure: the defaults and the checks of a new policy.

use chrono::{DateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use thiserror::Error;

/// The memory share of a node with no policy, in percent (placeholder: R-NEW-devicepolicy).
pub const DEFAULT_MEMORY_SHARE: u8 = 75;
/// The disk share for models of a node with no policy, in percent (placeholder:
/// R-NEW-devicepolicy).
pub const DEFAULT_DISK_SHARE: u8 = 50;

/// The device policy of one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DevicePolicy {
    /// The largest part of the usable memory that models may use, 1 to 100 percent.
    pub memory_share: u8,
    /// The largest part of the disk that model files may use, 1 to 100 percent.
    pub disk_share: u8,
    /// The hours in which the node takes no new model work.
    pub quiet_hours: Option<QuietHours>,
    /// Model work only on mains power (CAP-011). On by default: a computer with no battery
    /// always reads mains, so the default changes nothing for it.
    pub mains_only: bool,
    /// No new model work while the operator is active at the node (CAP-012). Off by default.
    pub idle_only: bool,
}

impl Default for DevicePolicy {
    fn default() -> Self {
        Self {
            memory_share: DEFAULT_MEMORY_SHARE,
            disk_share: DEFAULT_DISK_SHARE,
            quiet_hours: None,
            mains_only: true,
            idle_only: false,
        }
    }
}

/// The quiet hours of a node: from `start` to `end` in the time zone `zone`. When `end` is not
/// after `start`, the hours go past midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuietHours {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub zone: Tz,
}

impl QuietHours {
    /// Whether `now` is inside the quiet hours; `start` is inside, `end` is not.
    #[must_use]
    pub fn contain(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&self.zone).time();
        let minute = |time: NaiveTime| time.hour() * 60 + time.minute();
        let (start, end, at) = (minute(self.start), minute(self.end), minute(local));
        if start < end {
            start <= at && at < end
        } else {
            at >= start || at < end
        }
    }
}

/// Why a new policy is refused.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("the memory share must be 1 to 100 percent")]
    MemoryShare,
    #[error("the disk share must be 1 to 100 percent")]
    DiskShare,
    #[error("the quiet hours must start and end at different times")]
    EmptyQuietHours,
    #[error("a time must be HH:MM")]
    Time,
    #[error("the time zone is not an IANA name")]
    Zone,
}

impl DevicePolicy {
    /// The policy when its values are in range.
    ///
    /// # Errors
    /// [`PolicyError`] names the first value out of range.
    pub fn checked(self) -> Result<Self, PolicyError> {
        if !(1..=100).contains(&self.memory_share) {
            return Err(PolicyError::MemoryShare);
        }
        if !(1..=100).contains(&self.disk_share) {
            return Err(PolicyError::DiskShare);
        }
        if self
            .quiet_hours
            .is_some_and(|hours| hours.start == hours.end)
        {
            return Err(PolicyError::EmptyQuietHours);
        }
        Ok(self)
    }
}

/// Quiet hours from the text `HH:MM`, `HH:MM` and an IANA time zone.
///
/// # Errors
/// [`PolicyError::Time`] or [`PolicyError::Zone`].
pub fn quiet_hours(start: &str, end: &str, zone: &str) -> Result<QuietHours, PolicyError> {
    let time =
        |text: &str| NaiveTime::parse_from_str(text.trim(), "%H:%M").map_err(|_| PolicyError::Time);
    Ok(QuietHours {
        start: time(start)?,
        end: time(end)?,
        zone: zone.trim().parse().map_err(|_| PolicyError::Zone)?,
    })
}
