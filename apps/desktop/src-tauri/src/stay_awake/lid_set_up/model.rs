//! Shared lid capability and setup errors.
use serde::Serialize;

/// Whether this computer can keep awake with the lid closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lid {
    Ready,
    /// A Mac without the rule file.
    #[cfg(unix)]
    NeedsSetUp,
    /// The desktop decides the lid itself and ignores a different program (KDE Plasma).
    Unsupported,
}

/// Why a set-up step did not finish. A cancelled password dialog is `Refused` too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SetUpError {
    #[error("this system has no set-up step")]
    Unsupported,
    #[error("the administrator step was cancelled or failed")]
    Refused,
}
