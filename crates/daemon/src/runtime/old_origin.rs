//! The move of a daemon that was enrolled with the old link origin (`http://127.0.0.1:<port>`,
//! before R-172) to the socket of the link: its token record and then its configuration get
//! `unix:<path>`, once the program at that socket passes the check of the peer (R-NEW-socketpeer).
//! After the move the path is kept with the token, so no value of the environment decides it
//! again. The move needs no person: an existing computer heals at a start of the daemon.

use std::path::Path;

use crate::config::Config;
use crate::config::home::{Home, HomeError};
use crate::config::origin::{Origin, Reach};
use crate::hub::link_admits;
use crate::hub::peer::Expected;
use crate::token_store::{Credential, TokenStore, TokenStoreError};

/// The configuration and the stored credential of an enrolled daemon.
pub struct Enrolled {
    pub config: Config,
    pub credential: Credential,
}

#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    #[error("{0}")]
    Token(#[from] TokenStoreError),
    #[error("{0}")]
    Home(#[from] HomeError),
}

/// What the move does for an enrolment, from its two origins and the socket of the link.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// Not an old enrolment, or one whose two origins differ: nothing to move.
    Stay,
    /// Both origins are old: move them when the peer at the socket passes the check.
    Ask(Origin),
    /// The token record already names the socket (a move that stopped after its first write):
    /// write the configuration.
    Finish(Origin),
}

fn step(config: &Origin, credential: &Origin, socket: &Path) -> Step {
    if config.reach() != Reach::OwnLinkSocket {
        return Step::Stay;
    }
    let Ok(origin) = Origin::parse(&format!("unix:{}", socket.display())) else {
        return Step::Stay;
    };
    if *credential == origin {
        Step::Finish(origin)
    } else if credential == config {
        Step::Ask(origin)
    } else {
        Step::Stay
    }
}

impl Enrolled {
    /// Whether this is an old enrolment that a move may change.
    #[must_use]
    pub fn is_old(&self) -> bool {
        self.config.hub.reach() == Reach::OwnLinkSocket
    }

    /// The enrolment after the move to `socket`, when `expected` admits the program there. An
    /// enrolment that does not move comes back unchanged; nothing was written for it. A program
    /// that does not pass the check gets no byte (the check writes nothing to the socket).
    ///
    /// # Errors
    /// [`MoveError`] when the token record or the configuration cannot be written. The token
    /// record is written first, so a stop between the two writes is finished at the next start.
    pub async fn moved(
        self,
        home: &Home,
        store: &TokenStore,
        link: (&Path, &Expected),
    ) -> Result<Self, MoveError> {
        let (socket, expected) = link;
        let (origin, token_moves) = match step(&self.config.hub, &self.credential.hub, socket) {
            Step::Stay => return Ok(self),
            Step::Finish(origin) => (origin, false),
            Step::Ask(origin) => {
                // A socket that fails its checks is reported by the client of the run, which
                // reaches the same path; here it only means that nothing moves.
                if !matches!(link_admits(socket, expected).await, Ok(true)) {
                    return Ok(self);
                }
                (origin, true)
            }
        };
        let credential = Credential {
            hub: origin.clone(),
            ..self.credential
        };
        if token_moves {
            store.save(&credential)?;
        }
        let config = Config {
            hub: origin,
            ..self.config
        };
        home.write_config(&config)?;
        Ok(Self { config, credential })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOCKET: &str = "/Users/me/Library/Application Support/anima-daemon/link.sock";

    fn old() -> Origin {
        Origin::parse_kept("http://127.0.0.1:4100").expect("an old link origin")
    }

    fn moved_to() -> Origin {
        Origin::parse(&format!("unix:{SOCKET}")).expect("a link origin")
    }

    #[test]
    fn only_an_old_enrolment_with_the_same_origin_twice_asks_the_peer() {
        assert_eq!(
            step(&old(), &old(), Path::new(SOCKET)),
            Step::Ask(moved_to())
        );
        let https = Origin::parse("https://hub.example").expect("an origin");
        assert_eq!(step(&https, &https, Path::new(SOCKET)), Step::Stay);
        assert_eq!(
            step(&moved_to(), &moved_to(), Path::new(SOCKET)),
            Step::Stay
        );
        assert_eq!(step(&old(), &https, Path::new(SOCKET)), Step::Stay);
    }

    #[test]
    fn a_move_that_stopped_after_the_token_record_is_finished_without_a_new_check() {
        assert_eq!(
            step(&old(), &moved_to(), Path::new(SOCKET)),
            Step::Finish(moved_to())
        );
    }

    #[test]
    fn a_socket_path_that_is_no_link_origin_is_not_moved_to() {
        assert_eq!(
            step(&old(), &old(), Path::new("relative/link.sock")),
            Step::Stay
        );
    }
}
