//! The VPN as the hub's own node sees it (VPN-005, VPN-008 to VPN-011), and the broker that
//! makes keys and approves routes on the VPS (R-055, VP-7).

mod broker;
mod feed;
pub mod graphql;
mod localapi;
mod model;
mod peers;
mod picture;
mod reader;
mod steps;
mod store;
#[cfg(test)]
mod tests;
mod waiting;
mod warnings;

pub use broker::{Broker, Refusal};
pub use feed::Feed;
pub use model::{BrokerState, KeyState, Picture, Role, RouteState, SitePath, VpnNode, VpnPath};
pub use reader::{Reading, spawn_reader};

/// Why a read of the VPN status or a broker call failed. The text never holds a reply
/// (LOG-007).
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
    #[error("the broker socket is not there")]
    NotInstalled,
    #[error("the broker refused the request: {refusal}")]
    BrokerRefused {
        refusal: Refusal,
        /// The broker's own sentence, cleaned; never a key.
        detail: String,
    },
    #[error("the broker reply is empty, longer than 64 KiB or not the expected shape")]
    BrokerUnreadable,
    #[error("the broker did not answer in 65 seconds")]
    BrokerTimedOut,
    /// Another call of the hub held the turn; this one sent nothing (BD-15).
    #[error("another broker call holds the turn")]
    BrokerBusy,
    #[error("the broker connection failed")]
    BrokerConnection,
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
            Self::NotInstalled => "broker_not_installed",
            Self::BrokerRefused { .. } => "broker_refused",
            Self::BrokerUnreadable => "broker_unreadable",
            Self::BrokerTimedOut => "broker_timed_out",
            Self::BrokerBusy => "broker_busy",
            Self::BrokerConnection => "broker_connection",
            Self::Devices(_) => "devices",
            Self::Checks(_) => "checks",
            Self::Events(_) => "events",
            Self::Database(_) => "database",
        }
    }
}
