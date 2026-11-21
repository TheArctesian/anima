//! Who listens at the socket of `anima-link`: the check of the peer of each connection, made
//! before the daemon writes a byte to it (K8 Section 1, HMD-054).
//!
//! The kernel gives the user of the peer, and its process id. The user must be the daemon's own.
//! On macOS a daemon with a Developer ID signature also requires that the process of the peer is
//! `anima-link` with a Developer ID signature of the same team ([`signature`]). What this does
//! not close: a program of the same user that makes the socket, gives it to a process of its own
//! and lets the signed link take the process id (`exec`), or waits until a process id that it
//! knows goes to the signed link, or starts the signed link with settings of its own. The ruling
//! that chose this check says so.

mod signature;

use std::io;

pub use signature::TeamId;

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("the program at the socket of anima-link runs as a different user")]
    OtherUser,
    #[error("the program at the socket of anima-link is not the signed anima-link")]
    NotTheLink,
    #[error("the program at the socket of anima-link cannot be identified: {0}")]
    Io(#[from] io::Error),
    #[error(
        "the signature of this daemon cannot be read, or the file of the daemon is not the program that runs"
    )]
    OwnSignature,
}

/// What the program at the socket must be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Program {
    /// Any program of the user: a daemon with no Developer ID signature (a build for
    /// development), and each daemon on Linux, where the token file is open to the programs of
    /// the user in any case (K8 Section 2).
    OfTheUser,
    /// `anima-link` with a Developer ID signature of this team (macOS only).
    SignedBy(TeamId),
}

/// The peer that the daemon accepts at the socket of the link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    pub user: u32,
    pub program: Program,
}

/// The user and the process at the other end of one connection, as the kernel gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Peer {
    pub user: u32,
    pub process: Option<i32>,
}

impl Expected {
    /// The peer for this daemon: its own user, and on macOS the team of its own signature.
    ///
    /// # Errors
    /// [`PeerError::Io`] when the user cannot be read; [`PeerError::OwnSignature`] when the
    /// signature of this daemon cannot be read or its file is not the program that runs.
    pub fn of_this_daemon() -> Result<Self, PeerError> {
        let user = crate::config::private_file::running_user()?;
        #[cfg(target_os = "macos")]
        let program = signature::own_program()?;
        #[cfg(not(target_os = "macos"))]
        let program = Program::OfTheUser;
        Ok(Self { user, program })
    }

    /// Whether `peer` may get the request.
    ///
    /// # Errors
    /// [`PeerError::OtherUser`] or [`PeerError::NotTheLink`] when it may not; the request must
    /// then not be written.
    pub async fn admit(&self, peer: Peer) -> Result<(), PeerError> {
        if peer.user != self.user {
            return Err(PeerError::OtherUser);
        }
        match &self.program {
            Program::OfTheUser => Ok(()),
            Program::SignedBy(team) => {
                let process = peer.process.ok_or(PeerError::NotTheLink)?;
                if signature::is_the_link(process, team).await? {
                    Ok(())
                } else {
                    Err(PeerError::NotTheLink)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: u32 = 501;

    fn peer(user: u32) -> Peer {
        Peer {
            user,
            process: Some(1),
        }
    }

    #[tokio::test]
    async fn a_peer_of_a_different_user_is_refused_and_one_of_the_same_user_is_admitted() {
        let expected = Expected {
            user: ME,
            program: Program::OfTheUser,
        };
        assert!(matches!(
            expected.admit(peer(ME + 1)).await,
            Err(PeerError::OtherUser)
        ));
        assert!(expected.admit(peer(ME)).await.is_ok());
    }

    #[tokio::test]
    async fn a_signed_daemon_refuses_a_peer_whose_process_is_unknown() {
        let expected = Expected {
            user: ME,
            program: Program::SignedBy(TeamId::parse("ABCDE12345").expect("a team")),
        };
        let unknown = Peer {
            user: ME,
            process: None,
        };
        assert!(matches!(
            expected.admit(unknown).await,
            Err(PeerError::NotTheLink)
        ));
    }
}
