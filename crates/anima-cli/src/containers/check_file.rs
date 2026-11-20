//! `anima containers check-file <path>`: the check of the syntax of a Compose file (HOST-022).
//! The hub reads the text and reaches no device. It is a check of the syntax only: it does
//! not say that a machine accepts the file. A value of the file can be a secret, so a problem
//! gives a code, a position and at most the name of a service and of a key.

use std::path::PathBuf;

use super::documents::{ComposeFileCheckRun, compose_file_check_run};
use crate::failure::{Failure, exit};
use crate::hub::Hub;
use crate::input;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

/// The most bytes of a Compose file: the limit of the hub.
pub const LARGEST_FILE: usize = 256 * 1024;

/// What the check does not say, for each way to show its result.
pub const SYNTAX_ONLY: &str = "This is a check of the syntax only: it does not say that a \
                               machine accepts the file.";

pub type Check = compose_file_check_run::ComposeFileCheckRunComposeFileCheck;
type Severity = compose_file_check_run::ComposeFileProblemSeverity;

#[derive(Debug, clap::Args)]
pub struct CheckFileArgs {
    /// The Compose file, or `-` for standard input.
    file: PathBuf,
}

/// The result of the check of the text `file`.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(hub: &Hub, file: String) -> Result<Check, Failure> {
    let variables = compose_file_check_run::Variables { file };
    Ok(hub
        .run::<ComposeFileCheckRun>(variables)
        .await?
        .compose_file_check)
}

/// How many problems of the check are errors.
#[must_use]
pub fn errors(check: &Check) -> usize {
    let error = |severity: &Severity| matches!(severity, Severity::ERROR);
    let problems = check.problems.iter();
    problems.filter(|problem| error(&problem.severity)).count()
}

impl CheckFileArgs {
    /// Prints a row for each problem or, with `--json`, the K1 `ComposeFileCheck`.
    ///
    /// # Errors
    /// [`Failure::Input`] for a file that cannot be read, is not UTF-8 text or is larger than
    /// 256 KiB; the hub's refusal; [`Failure::Ended`] with exit status 1 when the check found
    /// an error.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let what = "the Compose file";
        let bytes = input::read(invocation.prompter, &self.file, what, LARGEST_FILE)?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|_| Failure::Input {
            what: what.to_owned(),
            reason: "it is not UTF-8 text".to_owned(),
        })?;
        let check = fetch(&invocation.hub()?, text).await?;
        if check.problems.is_empty() && !invocation.output.json() {
            invocation.output.line("No problem found.")?;
        } else {
            invocation.output.data(&check, table)?;
        }
        if check.cut {
            invocation
                .output
                .note("The file has more problems: only the first 100 are shown.");
        }
        invocation.output.note(SYNTAX_ONLY);
        match errors(&check) {
            0 => Ok(()),
            count => Err(Failure::Ended {
                message: format!("The Compose file has errors of syntax: {count}."),
                status: exit::FAILURE,
            }),
        }
    }
}

/// `NO_IMAGE_OR_BUILD` as `no image or build`.
fn words<T: serde::Serialize>(name: &T) -> String {
    enum_name(name).to_lowercase().replace('_', " ")
}

fn table(check: &Check) -> Table {
    let mut table = Table::new(vec![
        "LINE", "COLUMN", "SEVERITY", "PROBLEM", "SERVICE", "KEY",
    ]);
    for problem in &check.problems {
        let tone = match problem.severity {
            Severity::ERROR => Tone::Bad,
            _ => Tone::Warning,
        };
        let name = |name: &Option<String>| name.clone().unwrap_or_else(|| "-".to_owned());
        table.row(vec![
            problem.line.to_string().as_str().into(),
            problem.column.to_string().as_str().into(),
            Cell::toned(&words(&problem.severity), tone),
            words(&problem.code).as_str().into(),
            name(&problem.service).as_str().into(),
            name(&problem.key).as_str().into(),
        ]);
    }
    table
}
