//! The code signature of a process on macOS, as `/usr/bin/codesign` reads it: of the daemon
//! itself, and of the peer at the socket of the link.
//!
//! `codesign -d <pid>` reads the file at the path of the process, which a program of the user can
//! replace while the process runs. So each answer of `-d` is checked again by
//! `codesign --verify -R=<requirement> <pid>`, which fails when the file is not the program that
//! runs, or when the requirement does not hold.

use super::{PeerError, Program};
#[cfg(target_os = "macos")]
use std::{
    io::{self, Read as _},
    process::Stdio,
    time::{Duration, Instant},
};

/// `codesign` of the system, by its full path: the daemon runs it with an empty environment.
#[cfg(target_os = "macos")]
const CODESIGN: &str = "/usr/bin/codesign";

/// The longest one run of `codesign` may take.
#[cfg(target_os = "macos")]
const SIGNATURE_LIMIT: Duration = Duration::from_secs(5);

/// The signing identifier of the link program, alone or with the suffix that the signature of a
/// program with no bundle can add (`anima-link-<hex>`).
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
const LINK: &str = "anima-link";

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

/// The program that this daemon expects at the socket: `anima-link` of the team of its own
/// Developer ID signature, or any program of the user when it has none. The answer of
/// `codesign -d` is accepted only when `codesign --verify` of the running daemon agrees with it:
/// a file put in the place of the daemon makes an error, not a daemon with no signature.
#[cfg(target_os = "macos")]
pub(super) fn own_program() -> Result<Program, PeerError> {
    let me = std::process::id().to_string();
    let (shown, text) = codesign_now(&["-d".into(), "--verbose=2".into(), me.clone()])?;
    let program = shown_program(shown, &text).ok_or(PeerError::OwnSignature)?;
    let requirement = format!("-R={}", own_requirement(&program));
    let (verified, _) = codesign_now(&["--verify".into(), requirement, me])?;
    if verified {
        Ok(program)
    } else {
        Err(PeerError::OwnSignature)
    }
}

/// Whether the running process `process` is `anima-link` with a valid Developer ID signature of
/// `team`, with its file the program that runs. A run of `codesign` that takes too long is a no.
#[cfg(target_os = "macos")]
pub(super) async fn is_the_link(process: i32, team: &TeamId) -> Result<bool, PeerError> {
    if process <= 0 {
        return Ok(false);
    }
    let pid = process.to_string();
    let Some((shown, text)) = codesign(&["-d", "--verbose=2", &pid]).await? else {
        return Ok(false);
    };
    let Some(identifier) = shown.then(|| link_identifier(&text)).flatten() else {
        return Ok(false);
    };
    let requirement = format!("-R={}", link_requirement(identifier, team));
    Ok(codesign(&["--verify", &requirement, &pid])
        .await?
        .is_some_and(|(verified, _)| verified))
}

/// No signature is read outside macOS: [`Program::SignedBy`] admits nothing there.
#[cfg(not(target_os = "macos"))]
#[expect(clippy::unused_async, reason = "the same signature as the macOS check")]
pub(super) async fn is_the_link(_process: i32, _team: &TeamId) -> Result<bool, PeerError> {
    Ok(false)
}

/// One run of `codesign` with `args`, for a request: its success and its standard error, `None`
/// when it took longer than [`SIGNATURE_LIMIT`] (it is then ended).
#[cfg(target_os = "macos")]
async fn codesign(args: &[&str]) -> io::Result<Option<(bool, String)>> {
    let mut command = tokio::process::Command::new(CODESIGN);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    match tokio::time::timeout(SIGNATURE_LIMIT, command.output()).await {
        Ok(output) => {
            let output = output?;
            let text = String::from_utf8_lossy(&output.stderr).into_owned();
            Ok(Some((output.status.success(), text)))
        }
        Err(_) => Ok(None),
    }
}

/// One run of `codesign` with `args` at the start of the daemon, outside a request: its success
/// and its standard error; an error when it took longer than [`SIGNATURE_LIMIT`].
#[cfg(target_os = "macos")]
fn codesign_now(args: &[String]) -> io::Result<(bool, String)> {
    let mut child = std::process::Command::new(CODESIGN)
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let end = Instant::now() + SIGNATURE_LIMIT;
    loop {
        if let Some(status) = child.try_wait()? {
            let mut text = String::new();
            if let Some(mut errors) = child.stderr.take() {
                errors.read_to_string(&mut text)?;
            }
            return Ok((status.success(), text));
        }
        if Instant::now() >= end {
            // The time limit is the error that the caller gets; a failed kill or wait of a
            // program that is already ending adds nothing to it.
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::ErrorKind::TimedOut.into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The program to expect from what `codesign -d --verbose=2` showed of this daemon (its lines
/// are on standard error): [`Program::OfTheUser`] for no Developer ID signature (none at all, ad
/// hoc, or no team), `None` when the output does not say.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
fn shown_program(success: bool, shown: &str) -> Option<Program> {
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

/// What `codesign --verify` of the running daemon must find for `program`: the Developer ID of
/// the team, or no Apple anchor at all. The second fails for a daemon with a Developer ID
/// signature whose file shows none, and for a daemon with no valid signature.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
fn own_requirement(program: &Program) -> String {
    match program {
        Program::SignedBy(team) => team.requirement(),
        Program::OfTheUser => "! anchor apple generic".to_owned(),
    }
}

/// The signing identifier that `codesign -d` showed, when it is the one of the link program.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
fn link_identifier(shown: &str) -> Option<&str> {
    let identifier = shown
        .lines()
        .find_map(|line| line.strip_prefix("Identifier="))?
        .trim();
    let suffix_ok = |rest: &str| {
        rest.strip_prefix('-')
            .is_some_and(|hex| !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    let named = identifier
        .strip_prefix(LINK)
        .is_some_and(|rest| rest.is_empty() || suffix_ok(rest));
    named.then_some(identifier)
}

/// The requirement of the link program with `identifier` and a Developer ID signature of `team`.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "read only on macOS and by its tests")
)]
fn link_requirement(identifier: &str, team: &TeamId) -> String {
    format!("identifier \"{identifier}\" and {}", team.requirement())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team() -> TeamId {
        TeamId::parse("ABCDE12345").expect("a team")
    }

    #[test]
    fn only_the_form_of_a_team_id_goes_into_a_requirement() {
        assert!(TeamId::parse("ABCDE12345").is_some());
        for bad in ["abcde12345", "ABCDE1234", "ABCDE123456", "ABCDE\"1234", ""] {
            assert_eq!(TeamId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn what_the_daemon_shows_of_itself_names_the_program_to_expect_or_nothing() {
        let signed = "Executable=/x\nIdentifier=anima-daemon\nTeamIdentifier=ABCDE12345\n";
        assert_eq!(shown_program(true, signed), Some(Program::SignedBy(team())));
        let ad_hoc = "Identifier=anima_daemon-55\nSignature=adhoc\nTeamIdentifier=not set\n";
        assert_eq!(shown_program(true, ad_hoc), Some(Program::OfTheUser));
        assert_eq!(
            shown_program(false, "1234: code object is not signed at all\n"),
            Some(Program::OfTheUser)
        );
        assert_eq!(shown_program(false, "codesign: no such process\n"), None);
        assert_eq!(shown_program(true, "Identifier=x\n"), None);
        assert_eq!(shown_program(true, "TeamIdentifier=bad\"team\n"), None);
    }

    #[test]
    fn a_daemon_that_shows_no_signature_must_run_with_no_apple_anchor() {
        assert_eq!(
            own_requirement(&Program::OfTheUser),
            "! anchor apple generic"
        );
        assert_eq!(
            own_requirement(&Program::SignedBy(team())),
            team().requirement()
        );
    }

    #[test]
    fn only_the_identifier_of_the_link_program_is_accepted_and_goes_into_the_requirement() {
        let shown =
            |id: &str| format!("Executable=/x\nIdentifier={id}\nTeamIdentifier=ABCDE12345\n");
        assert_eq!(link_identifier(&shown("anima-link")), Some("anima-link"));
        assert_eq!(
            link_identifier(&shown("anima-link-5555494489debf33")),
            Some("anima-link-5555494489debf33")
        );
        for other in [
            "anima-daemon",
            "anima-linker",
            "anima-link-",
            "anima-link-xyz",
            "anima-link\" or anchor apple",
        ] {
            assert_eq!(link_identifier(&shown(other)), None, "{other}");
        }
        assert_eq!(link_identifier("TeamIdentifier=ABCDE12345\n"), None);
        assert!(
            link_requirement("anima-link", &team()).starts_with("identifier \"anima-link\" and ")
        );
    }
}
