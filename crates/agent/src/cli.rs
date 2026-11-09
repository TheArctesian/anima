//! The command line: `anima-agent enrol <code|-> [--hub <origin>]` and `anima-agent run`
//! (MET-012, MET-019).

use agent_core::wire::EnrolmentCode;

use crate::config::origin::{Origin, OriginError};

pub const USAGE: &str =
    "usage: anima-agent enrol <code | -> [--hub <https origin>]\n       anima-agent run";

/// Printed when the code is on the command line, where `ps` and the shell history show it.
pub const ARGV_WARNING: &str = "warning: a code on the command line shows in process lists and shell history; use: printf '%s\\n' '<code>' | anima-agent enrol -";

/// What the operator asked for.
pub enum Command {
    /// Exchange an enrolment code for a token and store both (K8 Section 3).
    Enrol { code: CodeSource, hub: Origin },
    /// Sample and push until stopped (K8 Sections 4 to 7).
    Run,
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
        _ => Err(CliError::Usage),
    }
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
}
