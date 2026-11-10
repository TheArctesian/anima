//! The command line: `anima-agent enrol <code|-> [--hub <origin>]`, `anima-agent run` and
//! `anima-agent install [--system]` (MET-012, MET-019).

use std::io::{BufRead, Read as _};

use agent_core::wire::EnrolmentCode;
use zeroize::Zeroizing;

use crate::config::origin::{Origin, OriginError};
use crate::units::plan::Target;

pub const USAGE: &str = "usage: anima-agent enrol <code | -> [--hub <https origin>]\n       anima-agent run\n       anima-agent install [--system]";

/// Printed when the code is on the command line, where `ps` and the shell history show it.
pub const ARGV_WARNING: &str = "warning: a code on the command line shows in process lists and shell history; use: printf '%s\\n' '<code>' | anima-agent enrol -";

/// What the operator asked for.
pub enum Command {
    /// Exchange an enrolment code for a token and store both (K8 Section 3).
    Enrol { code: CodeSource, hub: Origin },
    /// Sample and push until stopped (K8 Sections 4 to 7).
    Run,
    /// Write the unit that starts `run` at login or boot, and start it (MET-019, D-10).
    Install { target: Target },
}

/// Where the enrolment code comes from: the command line, or standard input for `-`.
pub enum CodeSource {
    Given(EnrolmentCode),
    Stdin,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CliError {
    #[error("{USAGE}")]
    Usage,
    #[error("that is not an enrolment code (it starts with anima_enrol_)")]
    NotACode,
    #[error("--hub: {0}")]
    Hub(#[from] OriginError),
}

/// Reads the arguments after the program name.
///
/// # Errors
/// [`CliError`] for a command, option or code that does not read. No error holds the code.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, CliError> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("run") if args.next().is_none() => Ok(Command::Run),
        Some("enrol") => enrol(args),
        Some("install") => install(args),
        _ => Err(CliError::Usage),
    }
}

fn install(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let target = match args.next().as_deref() {
        None => Target::User,
        Some("--system") => Target::System,
        Some(_) => return Err(CliError::Usage),
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    Ok(Command::Install { target })
}

fn enrol(args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let mut code = None;
    let mut hub = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--hub" {
            hub = Some(args.next().ok_or(CliError::Usage)?);
        } else if let Some(text) = arg.strip_prefix("--hub=") {
            hub = Some(text.to_owned());
        } else if code.is_none() && (arg == "-" || !arg.starts_with("--")) {
            code = Some(arg);
        } else {
            return Err(CliError::Usage);
        }
    }
    let code = match code.ok_or(CliError::Usage)?.as_str() {
        "-" => CodeSource::Stdin,
        text => CodeSource::Given(EnrolmentCode::parse(text).ok_or(CliError::NotACode)?),
    };
    let hub = match hub {
        Some(text) => Origin::parse(&text)?,
        None => Origin::default_hub(),
    };
    Ok(Command::Enrol { code, hub })
}

/// The most bytes read for a code from standard input: a code is under 40.
const CODE_INPUT_LIMIT: usize = 256;

/// The enrolment code on the first line of `input`: at most 256 bytes are read, and the line
/// may end in LF or CRLF.
///
/// # Errors
/// [`CliError::NotACode`] for an empty line, a line that is not a code (a code split by a
/// newline leaves only its first part), or 256 bytes with no newline. The error holds none of
/// the input.
///
/// The code also stays in the `StdinLock`'s own buffer, which is not wiped (LOG-007); the
/// process is short-lived, and only reads the one line.
pub fn read_code(mut input: impl BufRead) -> Result<EnrolmentCode, CliError> {
    // The buffer has room for every byte that can come, so it never moves and is wiped whole.
    let mut line = Zeroizing::new(String::with_capacity(CODE_INPUT_LIMIT + 1));
    input
        .by_ref()
        .take(CODE_INPUT_LIMIT as u64)
        .read_line(&mut line)
        .map_err(|_| CliError::NotACode)?;
    code_from_line(&line)
}

/// The code in `line`, which may end in one LF or CRLF (a terminal read gives none).
///
/// # Errors
/// [`CliError::NotACode`] when the rest is not a code.
pub fn code_from_line(line: &str) -> Result<EnrolmentCode, CliError> {
    let text = line.strip_suffix('\n').unwrap_or(line);
    let text = text.strip_suffix('\r').unwrap_or(text);
    EnrolmentCode::parse(text).ok_or(CliError::NotACode)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "anima_enrol_AAAAAAAAAAAAAAAAAAAAAA";

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn met_019_run_takes_no_arguments() {
        assert!(matches!(parse(args("run")), Ok(Command::Run)));
        assert!(matches!(parse(args("run now")), Err(CliError::Usage)));
        assert!(matches!(parse(args("")), Err(CliError::Usage)));
    }

    #[test]
    fn met_019_install_is_for_the_user_unless_system_is_given() {
        assert!(matches!(
            parse(args("install")),
            Ok(Command::Install {
                target: Target::User
            })
        ));
        assert!(matches!(
            parse(args("install --system")),
            Ok(Command::Install {
                target: Target::System
            })
        ));
        for line in ["install --user", "install --system now", "install system"] {
            assert!(matches!(parse(args(line)), Err(CliError::Usage)), "{line}");
        }
    }

    #[test]
    fn met_012_enrol_defaults_the_hub_and_reads_a_code_or_stdin() {
        let Ok(Command::Enrol { code, hub }) = parse(args(&format!("enrol {CODE}"))) else {
            panic!("enrol with a code reads");
        };
        assert!(matches!(code, CodeSource::Given(_)));
        assert_eq!(hub, Origin::default_hub());
        assert!(matches!(
            parse(args("enrol -")),
            Ok(Command::Enrol {
                code: CodeSource::Stdin,
                ..
            })
        ));
    }

    #[test]
    fn met_020_enrol_takes_an_https_hub_in_either_spelling_and_refuses_http() {
        for line in [
            "enrol - --hub https://hub.example",
            "enrol --hub=https://hub.example -",
        ] {
            let Ok(Command::Enrol { hub, .. }) = parse(args(line)) else {
                panic!("{line}");
            };
            assert_eq!(hub.as_text(), "https://hub.example");
        }
        assert!(matches!(
            parse(args("enrol - --hub http://hub.example")),
            Err(CliError::Hub(OriginError::NotHttps))
        ));
    }

    #[test]
    fn met_012_a_bad_code_is_refused_without_echoing_it() {
        let Err(error) = parse(args("enrol anima_enrol_short")) else {
            panic!("a short code is refused");
        };
        assert_eq!(error, CliError::NotACode);
        assert!(!error.to_string().contains("short"));
    }

    fn read(input: &[u8]) -> Result<EnrolmentCode, CliError> {
        read_code(input)
    }

    #[test]
    fn met_012_a_code_on_standard_input_ends_at_its_line_feed_or_carriage_return_and_line_feed() {
        for input in [
            format!("{CODE}\n"),
            format!("{CODE}\r\n"),
            CODE.to_owned(),
            format!("{CODE}\nmore"),
        ] {
            assert!(read(input.as_bytes()).is_ok(), "{input:?}");
        }
    }

    #[test]
    fn met_012_an_empty_a_split_or_a_padded_line_is_not_a_code() {
        let (head, tail) = CODE.split_at(10);
        for input in [
            String::new(),
            "\n".to_owned(),
            "\r\n".to_owned(),
            format!("{head}\n{tail}\n"),
            format!(" {CODE}\n"),
            format!("{CODE}\r\r\n"),
        ] {
            assert_eq!(
                read(input.as_bytes()).err(),
                Some(CliError::NotACode),
                "{input:?}"
            );
        }
    }

    #[test]
    fn met_012_at_most_256_bytes_are_read_and_300_without_a_newline_are_not_a_code() {
        let long = vec![b'A'; 300];
        let mut input = std::io::Cursor::new(long);
        assert_eq!(read_code(&mut input).err(), Some(CliError::NotACode));
        assert_eq!(input.position(), 256, "the rest was never read");
    }
}
