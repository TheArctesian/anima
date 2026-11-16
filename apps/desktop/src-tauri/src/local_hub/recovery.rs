//! Recovery of the operator account (F-9): `account create` replaces an existing account only
//! after the person at this computer confirmed that one request in a native dialog, which
//! script in the web view cannot answer. Each replacement takes its own confirmation; nothing
//! is held for a later request. Pure: the dialog is a parameter.

use std::time::{Duration, Instant};

use super::LocalHubError;

/// The text of the confirmation: what will happen.
pub const CONFIRMATION_TITLE: &str = "Recover the account";
pub const CONFIRMATION: &str = "This sets a new password and a new authenticator for the \
operator account of the hub on this computer. Every sign-in to the hub ends, on every device.\n\n\
Continue only if you asked for this yourself.";

/// How long after a cancelled confirmation no new one is shown, so script cannot raise the
/// dialog again and again until a person clicks it by mistake.
pub const PAUSE_AFTER_CANCEL: Duration = Duration::from_mins(1);

/// When the last confirmation was cancelled.
#[derive(Debug, Default)]
pub struct Cancellations {
    last: Option<Instant>,
}

/// Whether `account create` may run: at once on the first run, when the hub has no account;
/// otherwise only when `confirm` (the native dialog) answers yes. `confirm` runs only then, and
/// not within [`PAUSE_AFTER_CANCEL`] of a cancel (`busy`).
pub fn may_create(
    has_account: bool,
    cancellations: &mut Cancellations,
    now: Instant,
    confirm: impl FnOnce() -> bool,
) -> Result<(), LocalHubError> {
    if !has_account {
        return Ok(());
    }
    let paused = cancellations
        .last
        .is_some_and(|last| now.saturating_duration_since(last) < PAUSE_AFTER_CANCEL);
    if paused {
        return Err(LocalHubError::Busy);
    }
    if !confirm() {
        cancellations.last = Some(now);
        return Err(LocalHubError::Cancelled);
    }
    Ok(())
}
