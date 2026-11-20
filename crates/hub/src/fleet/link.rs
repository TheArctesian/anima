//! The device link (one-click plan, items S5 and S6): the hub makes invites, lists the devices
//! of `anima-link hub` and revokes one, through the link's control socket (FRN-009, LNK-003,
//! LNK-004). `ANIMA_LINK_CONTROL` names the socket; without it the fields are `UNAVAILABLE`.
//!
//! [`presence`] says which devices of the link are connected now (HMD-047).
//!
//! The link is a separate program of the same user. Its replies are hostile input: each is
//! read to at most 1 MiB, parsed once, and never echoed into an error or a log, because an
//! invite reply holds a secret (LOG-007).

mod claims;
pub mod graphql;
mod model;
#[cfg(unix)]
mod owned;
mod presence;
mod socket;
#[cfg(all(test, unix))]
mod tests;
mod wire;

pub use claims::Claims;
pub use model::LinkPath;
pub(crate) use model::Refusal;
pub use presence::{Presence, presence};

/// Why a call to the link failed. No variant holds a reply, an invite or a key (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// `ANIMA_LINK_CONTROL` is not set: the hub has no link to ask.
    #[error("the hub has no link control socket")]
    Off,
    #[error("the link's control socket is not there")]
    NotRunning,
    /// The socket, its folder or a folder above it is another user's or open to others; the
    /// hub sends nothing there (the rule of `link/statedir`).
    #[error("the link's control socket is exposed: {0}")]
    Exposed(&'static str),
    #[error("the link did not answer in time")]
    TimedOut,
    #[error("the connection to the link failed")]
    Connection,
    #[error("the link's reply is empty, longer than 1 MiB or not the expected shape")]
    Unreadable,
    #[error("the link refused the request: {0}")]
    Refused(Refusal),
}

impl Error {
    /// A fixed word for logs; never the reply text (LOG-007).
    #[must_use]
    pub const fn fault(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::NotRunning => "not_running",
            Self::Exposed(_) => "exposed",
            Self::TimedOut => "timed_out",
            Self::Connection => "connection",
            Self::Unreadable => "unreadable",
            Self::Refused(_) => "refused",
        }
    }
}
