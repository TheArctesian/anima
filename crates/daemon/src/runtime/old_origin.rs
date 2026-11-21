//! The move of a daemon that was enrolled with the old link origin (`http://127.0.0.1:<port>`,
//! before R-172) to the socket of the link: its token record and then its configuration get
//! `unix:<path>`, once the program at that socket passes the check of the peer (R-NEW-socketpeer).
//! After the move the path is kept with the token, so no value of the environment decides it
//! again. The move needs no person: an existing computer heals at a start of the daemon. Until
//! then the daemon sends through the socket in its own folder, to a peer that passes the same
//! check, and asks the socket of the move again each minute ([`Watch`]).

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::Config;
use crate::config::home::{Home, HomeError};
use crate::config::origin::{Origin, Reach};
use crate::hub::link_admits;
use crate::hub::peer::{Expected, PeerError};
use crate::report;
use crate::token_store::{Credential, TokenStore, TokenStoreError};

/// How often a daemon whose old enrolment did not move asks the socket of the move again.
pub const ASK_EVERY: Duration = Duration::from_mins(1);

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
    #[error("{0}")]
    Peer(#[from] PeerError),
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
    // A path that is not text would be written as a different one.
    let Some(Ok(origin)) = socket
        .to_str()
        .map(|path| Origin::parse(&format!("unix:{path}")))
    else {
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
    /// With a [`MoveError`] when the token record or the configuration cannot be written; the
    /// enrolment then comes back as it was. The token record is written first, so a stop between
    /// the two writes is finished at the next start.
    pub async fn moved(
        self,
        home: &Home,
        store: &TokenStore,
        link: (&Path, &Expected),
    ) -> (Self, Option<MoveError>) {
        let (socket, expected) = link;
        let (origin, token_moves) = match step(&self.config.hub, &self.credential.hub, socket) {
            Step::Stay => return (self, None),
            Step::Finish(origin) => (origin, false),
            Step::Ask(origin) => {
                // A socket that fails its checks is reported by the client of the run; here it
                // only means that nothing moves.
                if !matches!(link_admits(socket, expected).await, Ok(true)) {
                    return (self, None);
                }
                (origin, true)
            }
        };
        let credential = Credential {
            hub: origin.clone(),
            ..self.credential
        };
        if token_moves && let Err(error) = store.save(&credential) {
            let credential = Credential {
                hub: self.config.hub.clone(),
                ..credential
            };
            return (Self { credential, ..self }, Some(error.into()));
        }
        let config = Config {
            hub: origin,
            ..self.config.clone()
        };
        if let Err(error) = home.write_config(&config) {
            // The record in memory keeps the origin of the configuration, which the client uses.
            let credential = Credential {
                hub: self.config.hub.clone(),
                ..credential
            };
            return (Self { credential, ..self }, Some(error.into()));
        }
        (Self { config, credential }, None)
    }
}

/// What the start of the daemon has for its run: the enrolment, after its move when the peer at
/// the socket of the move passes; the [`Watch`] of an old enrolment that did not move; and the
/// socket that the client of an old enrolment sends through until the move.
pub struct Start {
    pub enrolled: Enrolled,
    pub watch: Watch,
    pub socket: PathBuf,
}

/// The [`Start`] of the daemon. A move that cannot be written leaves the enrolment as it was,
/// and is tried again at the next start; the daemon goes on sending, to a peer that passes the
/// check. Until the move, an old enrolment sends through the socket of the move on macOS (no
/// value of the environment decides where a token from the Keychain goes), and through the
/// socket of its own folder on Linux, which is the same path unless the environment names
/// another folder.
///
/// # Errors
/// [`MoveError::Peer`] when the signature of this daemon cannot be read; [`MoveError::Home`] on
/// macOS when the home folder of the account cannot be read.
pub async fn at_start(
    home: &Home,
    store: &TokenStore,
    enrolled: Enrolled,
) -> Result<Start, MoveError> {
    let own = home.link_socket();
    if !enrolled.is_old() {
        return Ok(Start {
            enrolled,
            watch: Watch::none(),
            socket: own,
        });
    }
    let expected = Expected::of_this_daemon()?;
    let socket = socket_of_move(home).map_err(HomeError::from)?;
    let client = if cfg!(target_os = "macos") {
        socket.clone()
    } else {
        own
    };
    let (enrolled, failed) = enrolled.moved(home, store, (&socket, &expected)).await;
    let asks = matches!(
        step(&enrolled.config.hub, &enrolled.credential.hub, &socket),
        Step::Ask(_)
    );
    let watch = match failed {
        Some(error) => {
            report::move_failed(&error);
            Watch::none()
        }
        None if !enrolled.is_old() => {
            report::moved();
            Watch::none()
        }
        None => {
            report::old_origin_waits(&client);
            if asks {
                Watch::of(socket, expected, ASK_EVERY)
            } else {
                Watch::none()
            }
        }
    };
    Ok(Start {
        enrolled,
        watch,
        socket: client,
    })
}

/// The socket that an old enrolment moves to: on macOS below the home folder of the account,
/// not the folder that the environment names, because the token is in the Keychain; on Linux
/// in the daemon's own folder.
#[cfg_attr(
    not(target_os = "macos"),
    expect(
        clippy::unnecessary_wraps,
        reason = "the macOS form reads the user database, which can fail"
    )
)]
fn socket_of_move(home: &Home) -> std::io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let _ = home;
        crate::config::home::account_link_socket()
    }
    #[cfg(not(target_os = "macos"))]
    Ok(home.link_socket())
}

/// The wait of an old enrolment that did not move, for the program at the socket of the move to
/// pass the check of the peer. The daemon then starts again, and the move is made at the start.
pub struct Watch(Option<(PathBuf, Expected, Duration)>);

impl Watch {
    /// A wait that never ends: nothing to move.
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    /// A wait that asks the socket at `socket` every `every`.
    #[must_use]
    pub const fn of(socket: PathBuf, expected: Expected, every: Duration) -> Self {
        Self(Some((socket, expected, every)))
    }

    /// Ends when the program at the socket passes the check; never for [`Watch::none`].
    pub async fn passed(&self) {
        let Some((socket, expected, every)) = &self.0 else {
            return std::future::pending().await;
        };
        loop {
            tokio::time::sleep(*every).await;
            if matches!(link_admits(socket, expected).await, Ok(true)) {
                return;
            }
        }
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
        let not_text = Path::new(
            <std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(
                b"/Users/m\xffe/link.sock",
            ),
        );
        assert_eq!(step(&old(), &old(), not_text), Step::Stay);
    }
}
