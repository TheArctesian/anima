//! `anima vpn`: the VPN of the hub, its keys and its routes (CMD-002).

pub mod approve_route;
pub mod documents;
pub mod key;
pub mod registration;
pub mod status;
pub mod watch;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum VpnCommand {
    /// Show the hub, each site's path and every node, with key expiry.
    Status(status::StatusArgs),
    /// Make a one-hour, single-use key for a Linux computer of yours.
    Invite(key::InviteArgs),
    /// Make a one-hour, single-use key for the subnet router of a site.
    RouterKey(key::RouterKeyArgs),
    /// Register the computer that shows this auth ID.
    Register(registration::RegisterArgs),
    /// Give an untagged operator node a new key expiry.
    Renew(registration::RenewArgs),
    /// Approve the waiting route of a site's router.
    ApproveRoute(approve_route::ApproveRouteArgs),
}

impl VpnCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown site or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Status(args) => args.run(invocation).await,
            Self::Invite(args) => args.run(invocation).await,
            Self::RouterKey(args) => args.run(invocation).await,
            Self::Register(args) => args.run(invocation).await,
            Self::Renew(args) => args.run(invocation).await,
            Self::ApproveRoute(args) => args.run(invocation).await,
        }
    }
}
