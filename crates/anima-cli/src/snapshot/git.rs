//! The one edge of `anima snapshot`: `git` run as a child process in the top folder of the
//! working tree, with each argument passed as it is (never through a shell). Its only network
//! use is `git push` and `git ls-remote`; no hub request is made.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::SnapshotError;
use super::git_output::safe;

/// git in one working tree, with the variables every call of this instance gets.
#[derive(Clone, Debug)]
pub struct Git {
    top: PathBuf,
    variables: Vec<(&'static str, OsString)>,
}

/// What one call printed and whether it succeeded.
#[derive(Debug)]
pub struct Ran {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Git {
    /// git in the working tree that holds `folder`.
    ///
    /// # Errors
    /// [`SnapshotError::NotWorkingTree`] when `folder` is not in one (a bare repository, or no
    /// repository); [`SnapshotError::GitMissing`] when git cannot be started.
    pub fn open(folder: &Path) -> Result<Self, SnapshotError> {
        let probe = Self {
            top: folder.to_owned(),
            variables: Vec::new(),
        };
        let ran = probe.run(&["rev-parse", "--show-toplevel"])?;
        let top = String::from_utf8_lossy(&ran.stdout).trim_end().to_owned();
        if !ran.success || top.is_empty() {
            return Err(SnapshotError::NotWorkingTree);
        }
        Ok(Self {
            top: PathBuf::from(top),
            variables: Vec::new(),
        })
    }

    /// The same git with `name=value` set for each call.
    #[must_use]
    pub fn with(&self, name: &'static str, value: impl Into<OsString>) -> Self {
        let mut git = self.clone();
        git.variables.push((name, value.into()));
        git
    }

    /// Runs `git <args>` and keeps what it printed; a failure of git itself is in [`Ran`].
    ///
    /// # Errors
    /// [`SnapshotError::GitMissing`] when git cannot be started.
    pub fn run(&self, args: &[&str]) -> Result<Ran, SnapshotError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.top)
            .envs(self.variables.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::null())
            .output()
            .map_err(|error| SnapshotError::GitMissing {
                reason: error.to_string(),
            })?;
        Ok(Ran {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    /// Runs `git <args>` and returns standard output, which must be UTF-8 text.
    ///
    /// # Errors
    /// [`SnapshotError::Git`] when git fails, with its error text made safe (LOG-007).
    pub fn text(&self, args: &[&str]) -> Result<String, SnapshotError> {
        let bytes = self.bytes(args)?;
        String::from_utf8(bytes).map_err(|_| SnapshotError::Git {
            step: step_of(args),
            reason: "it printed text that is not UTF-8".to_owned(),
        })
    }

    /// Runs `git <args>` and returns standard output as bytes.
    ///
    /// # Errors
    /// As [`Git::text`].
    pub fn bytes(&self, args: &[&str]) -> Result<Vec<u8>, SnapshotError> {
        let ran = self.run(args)?;
        if ran.success {
            Ok(ran.stdout)
        } else {
            Err(SnapshotError::Git {
                step: step_of(args),
                reason: safe(&ran.stderr),
            })
        }
    }
}

/// The git subcommand of `args`, for an error.
fn step_of(args: &[&str]) -> String {
    args.first().copied().unwrap_or("git").to_owned()
}

/// `GIT_ALTERNATE_OBJECT_DIRECTORIES` with `objects` first, then what was set before. git
/// splits the list on `:` and reads an entry that starts with `"` as a C string, so a path
/// with `:`, `"`, `\` or a control character is written that way.
#[must_use]
pub fn alternates(objects: &str, before: Option<&str>) -> String {
    let first = if objects.contains([':', '"', '\\']) || objects.chars().any(char::is_control) {
        c_quoted(objects)
    } else {
        objects.to_owned()
    };
    match before.filter(|before| !before.is_empty()) {
        Some(before) => format!("{first}:{before}"),
        None => first,
    }
}

fn c_quoted(text: &str) -> String {
    let mut quoted = String::from("\"");
    for c in text.chars() {
        match c {
            '"' | '\\' => {
                quoted.push('\\');
                quoted.push(c);
            }
            c if c.is_ascii_control() => {
                let _ = write!(quoted, "\\{:03o}", u32::from(c));
            }
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_017_the_alternate_list_puts_the_repository_first_and_quotes_odd_paths() {
        assert_eq!(alternates("/r/.git/objects", None), "/r/.git/objects");
        assert_eq!(
            alternates("/r/.git/objects", Some("/x/objects")),
            "/r/.git/objects:/x/objects"
        );
        assert_eq!(alternates("/a:b/objects", Some("")), r#""/a:b/objects""#);
        assert_eq!(
            alternates("/a\"b\\c\nd/objects", None),
            r#""/a\"b\\c\012d/objects""#
        );
    }
}
