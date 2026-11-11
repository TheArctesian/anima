//! The VPN as the hub's own node sees it (VPN-005, VPN-008 to VPN-011).

mod model;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the reader (A1-T4) is the first caller outside the tests"
    )
)]
mod peers;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the reader (A1-T4) is the first caller outside the tests"
    )
)]
mod picture;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the reader (A1-T4) is the first caller outside the tests"
    )
)]
mod store;
#[cfg(test)]
mod tests;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the reader (A1-T4) is the first caller outside the tests"
    )
)]
mod warnings;

pub use model::{KeyState, Picture, Role, RouteState, SitePath, VpnNode, VpnPath};

/// Why a read of the VPN status failed. The text never holds the reply (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the LocalAPI proxy did not answer")]
    Request,
    #[error("the LocalAPI proxy answered {0}")]
    Status(u16),
    #[error("the LocalAPI proxy refused the request")]
    Refused,
    #[error("the status reply is longer than 1 MiB")]
    TooLong,
    #[error("the status reply is not the expected shape")]
    Unreadable,
    #[error(transparent)]
    Devices(#[from] crate::devices::Error),
    #[error(transparent)]
    Checks(#[from] crate::checks::Error),
    #[error(transparent)]
    Events(#[from] crate::events::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl Error {
    /// A fixed word for logs; never the reply text (LOG-007).
    #[must_use]
    pub const fn fault(&self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Status(_) => "status",
            Self::Refused => "refused",
            Self::TooLong => "too_long",
            Self::Unreadable => "unreadable",
            Self::Devices(_) => "devices",
            Self::Checks(_) => "checks",
            Self::Events(_) => "events",
            Self::Database(_) => "database",
        }
    }
}
