//! `anima link`: the devices of the hub's link (`anima-link`), their invites and their
//! revocation (FRN-009, LNK-003, LNK-004). An invite and a revocation need a login token
//! (D-23).

pub mod devices;
pub mod documents;
pub mod invite;
pub mod revoke;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum LinkCommand {
    /// Show the devices the link permits: name, fingerprint, path, expiry and open streams.
    Devices(devices::DevicesArgs),
    /// Make a single-use invite for a new device (a login token is needed).
    Invite(invite::InviteArgs),
    /// Remove a device from the link by its key and close its streams (a login token is needed).
    Revoke(revoke::RevokeArgs),
}

impl LinkCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, a hub with no link or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Devices(args) => args.run(invocation).await,
            Self::Invite(args) => args.run(invocation).await,
            Self::Revoke(args) => args.run(invocation).await,
        }
    }
}
