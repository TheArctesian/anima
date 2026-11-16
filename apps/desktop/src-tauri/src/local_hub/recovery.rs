//! The recovery permit (F-9): after the person at this computer confirmed recovery in a native
//! dialog, which script in the web view cannot answer, one `account create` may replace the
//! operator account for a short time. Pure: the time is a parameter.

use std::time::{Duration, Instant};

use super::LocalHubError;

/// How long a confirmation permits one replacement.
pub const PERMIT_LIFE: Duration = Duration::from_mins(5);

/// The text of the confirmation: what will happen.
pub const CONFIRMATION_TITLE: &str = "Recover the account";
pub const CONFIRMATION: &str = "This sets a new password and a new authenticator for the \
operator account of the hub on this computer. Every sign-in to the hub ends, on every device.\n\n\
Continue only if you asked for this yourself.";

/// Whether `account create` may run: on the first run, when the hub has no account, or with a
/// held permit. Otherwise the account `exists`.
pub fn may_create(has_account: bool, permit_held: bool) -> Result<(), LocalHubError> {
    if has_account && !permit_held {
        return Err(LocalHubError::Exists);
    }
    Ok(())
}

/// The one permit, held or not.
#[derive(Debug, Default)]
pub struct Recovery {
    granted: Option<Instant>,
}

impl Recovery {
    /// Holds a permit from `now` for [`PERMIT_LIFE`]; a permit held before is replaced.
    pub fn grant(&mut self, now: Instant) {
        self.granted = Some(now);
    }

    /// Whether a permit is held and has not expired at `now`.
    pub fn is_held(&self, now: Instant) -> bool {
        self.granted
            .is_some_and(|granted| now.saturating_duration_since(granted) < PERMIT_LIFE)
    }

    /// Ends the permit: it was used.
    pub fn consume(&mut self) {
        self.granted = None;
    }
}
