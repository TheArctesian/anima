//! Who listens at the socket of `anima-link`: the check of the peer of each connection, made
//! before the daemon writes a byte to it (K8 Section 1, HMD-054).
//!
//! The kernel gives the user of the peer, and its process id. The user must be the daemon's own.
//! On macOS a daemon with a Developer ID signature also asks `codesign` whether the process
//! of the peer has a Developer ID signature of the same team. What this does not close: a
//! program of the same user that makes the socket, gives it to a process of its own and lets a
//! signed program take the process id (`exec`), or that starts the signed link with settings of
//! its own. The ruling that chose this check says so.

use std::io;
#[cfg(target_os = "macos")]
use std::process::Stdio;
#[cfg(target_os = "macos")]
use std::time::Duration;

/// `codesign` of the system, by its full path: the daemon runs it with an empty environment.
#[cfg(target_os = "macos")]
const CODESIGN: &str = "/usr/bin/codesign";

/// The longest a check of a signature may take; the request has its own limit around it.
#[cfg(target_os = "macos")]
const SIGNATURE_LIMIT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("the program at the socket of anima-link runs as a different user")]
    OtherUser,
    #[error("the program at the socket of anima-link is not the signed anima-link")]
    NotTheLink,
    #[error("the program at the socket of anima-link cannot be identified: {0}")]
    Io(#[from] io::Error),
}

/// The team of a Developer ID signature: ten capital letters and digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamId(String);

impl TeamId {
    /// `text` when it has the form of a Team ID; `None` else, so that no other text goes into a
    /// code requirement.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let valid = text.len() == 10
            && text
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
        valid.then(|| Self(text.to_owned()))
    }

    /// The code requirement of a Developer ID Application certificate of this team: the same
    /// requirement that the builder of a release verifies (BLD-083).
    #[must_use]
    pub fn requirement(&self) -> String {
        format!(
            "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and \
             certificate leaf[field.1.2.840.113635.100.6.1.13] exists and \
             certificate leaf[subject.OU] = \"{}\"",
            self.0
        )
    }
}

/// What the program at the socket must be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Program {
    /// Any program of the user: a daemon with no Developer ID signature (a build for
    /// development), and each daemon on Linux, where the token file is open to the programs of
    /// the user in any case (K8 Section 2).
    OfTheUser,
    /// A program with a Developer ID signature of this team (macOS only).
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
    /// [`PeerError::Io`] when the user or the signature of this daemon cannot be read.
    pub fn of_this_daemon() -> Result<Self, PeerError> {
        let user = crate::config::private_file::running_user()?;
        #[cfg(target_os = "macos")]
        let program = own_program()?;
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
                if signed_by(process, team).await? {
                    Ok(())
                } else {
                    Err(PeerError::NotTheLink)
                }
            }
        }
    }
}

/// Whether the running process `process` has a valid Developer ID signature of `team`, as
/// `codesign --verify` finds it in the kernel's view of that process.
#[cfg(target_os = "macos")]
async fn signed_by(process: i32, team: &TeamId) -> Result<bool, PeerError> {
    if process <= 0 {
        return Ok(false);
    }
    let mut command = tokio::process::Command::new(CODESIGN);
    command
        .arg("--verify")
        .arg(format!("-R={}", team.requirement()))
        .arg(process.to_string())
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    match tokio::time::timeout(SIGNATURE_LIMIT, command.status()).await {
        Ok(status) => Ok(status?.success()),
        Err(_) => Ok(false),
    }
}

/// No signature check is made outside macOS: [`Program::SignedBy`] admits nothing there.
#[cfg(not(target_os = "macos"))]
#[expect(clippy::unused_async, reason = "the same signature as the macOS check")]
async fn signed_by(_process: i32, _team: &TeamId) -> Result<bool, PeerError> {
    Ok(false)
}

/// The program that this daemon expects, from `codesign -d` of the running process (not of the
/// file, which a program of the user can replace): a program of the team of its own Developer
/// ID signature, or any program of the user when it has none; an error when `codesign` cannot
/// say.
#[cfg(target_os = "macos")]
fn own_program() -> Result<Program, PeerError> {
    let shown = std::process::Command::new(CODESIGN)
        .args(["-d", "--verbose=2"])
        .arg(std::process::id().to_string())
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    let text = String::from_utf8_lossy(&shown.stderr);
    expected_program(shown.status.success(), &text).ok_or_else(|| {
        PeerError::Io(io::Error::other(
            "codesign did not show the signature of the daemon",
        ))
    })
}

/// The program to expect from what `codesign -d --verbose=2` showed of this daemon (its lines
/// are on standard error): [`Program::OfTheUser`] for no Developer ID signature (none at all, ad
/// hoc, or no team), `None` when the output does not say.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
fn expected_program(success: bool, shown: &str) -> Option<Program> {
    if !success {
        return shown
            .contains("code object is not signed at all")
            .then_some(Program::OfTheUser);
    }
    let team = shown
        .lines()
        .find_map(|line| line.strip_prefix("TeamIdentifier="))?
        .trim();
    if team == "not set" {
        return Some(Program::OfTheUser);
    }
    TeamId::parse(team).map(Program::SignedBy)
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

    #[test]
    fn only_the_form_of_a_team_id_goes_into_a_requirement() {
        assert!(TeamId::parse("ABCDE12345").is_some());
        for bad in ["abcde12345", "ABCDE1234", "ABCDE123456", "ABCDE\"1234", ""] {
            assert_eq!(TeamId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_team_comes_from_the_signature_and_an_unclear_answer_is_no_answer() {
        let signed = "Executable=/x\nIdentifier=anima-daemon\nTeamIdentifier=ABCDE12345\n";
        assert_eq!(
            expected_program(true, signed),
            TeamId::parse("ABCDE12345").map(Program::SignedBy)
        );
        let ad_hoc = "Identifier=anima_daemon-55\nSignature=adhoc\nTeamIdentifier=not set\n";
        assert_eq!(expected_program(true, ad_hoc), Some(Program::OfTheUser));
        assert_eq!(
            expected_program(false, "1234: code object is not signed at all\n"),
            Some(Program::OfTheUser)
        );
        assert_eq!(expected_program(false, "codesign: no such process\n"), None);
        assert_eq!(expected_program(true, "Identifier=x\n"), None);
        assert_eq!(expected_program(true, "TeamIdentifier=bad\"team\n"), None);
    }
}
