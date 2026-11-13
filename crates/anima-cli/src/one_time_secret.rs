//! A secret the hub shows once: an API key, an agent code, a VPN pre-auth key, a builder token.
//! Every command that has one shows it the same way (CMD-014, LOG-007): alone on standard
//! output as one line, or under its field with `--json`, or in a `0600` file with nothing
//! secret printed. The note that it is shown one time goes to standard error.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::clean;
use crate::private_file;

/// `--to-file` and `--force`: where a secret goes instead of standard output.
#[derive(Debug, Default, clap::Args)]
pub struct ToFile {
    /// Write the secret to this file (mode 0600) instead of standard output.
    #[arg(long = "to-file", value_name = "PATH")]
    path: Option<PathBuf>,
    /// Replace the --to-file file when it already exists.
    #[arg(long, requires = "path")]
    force: bool,
}

/// A secret a command shows once, and where it goes.
pub struct Secret<'a> {
    /// What the person calls it: "pre-auth key".
    pub label: &'a str,
    /// Its field in the `--json` answer.
    pub field: &'a str,
    pub value: &'a str,
    pub to_file: &'a ToFile,
}

/// Shows `secret` once: as a line on standard output, or in its file with nothing secret
/// printed. With `--json`, `public` is the answer, with the secret under its field unless it
/// went to a file. The notes go to standard error.
///
/// # Errors
/// [`Failure::Write`] when the file or standard output cannot be written; [`Failure::Usage`]
/// when the file is there and `--force` is not set.
pub fn show_once(
    invocation: &Invocation,
    secret: &Secret<'_>,
    mut public: Value,
) -> Result<(), Failure> {
    if let Some(path) = secret.to_file.path.as_deref() {
        write_file(path, secret)?;
        invocation.output.note(&format!(
            "The {} is in {} (mode 0600). It is shown one time and works once.",
            secret.label,
            path.display()
        ));
        if invocation.output.json() {
            public["writtenTo"] = json!(path.display().to_string());
            invocation.output.raw(&public)?;
        }
        return Ok(());
    }
    invocation.output.note(&format!(
        "The {} below is shown one time and works once. Keep it out of command lines and logs.",
        secret.label
    ));
    if invocation.output.json() {
        public[secret.field] = json!(secret.value);
        return invocation.output.raw(&public);
    }
    invocation.output.line(&clean(secret.value))
}

fn write_file(path: &Path, secret: &Secret<'_>) -> Result<(), Failure> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Failure::Usage {
            message: "--to-file needs a path that ends in a file name.".to_owned(),
            hint: None,
        })?;
    let bytes = secret.value.as_bytes();
    let saved = if secret.to_file.force {
        private_file::write(dir, name, bytes)
    } else {
        private_file::write_new(dir, name, bytes)
    };
    saved.map_err(|error| match error {
        private_file::PrivateFileError::Io(io)
            if io.kind() == std::io::ErrorKind::AlreadyExists =>
        {
            exists(path)
        }
        error => Failure::Write {
            reason: format!(
                "{}: {error}. The {} is lost; make another",
                path.display(),
                secret.label
            ),
        },
    })
}

fn exists(path: &Path) -> Failure {
    Failure::Conflict {
        message: format!(
            "{} already exists; pass --force to replace it.",
            path.display()
        ),
    }
}

impl ToFile {
    /// Fails before the hub is asked for a secret when the file is there and `--force` is not
    /// set: a secret made for a file that cannot be written would be lost.
    ///
    /// # Errors
    /// [`Failure::Conflict`] when `--to-file` names an existing file.
    pub fn check(&self) -> Result<(), Failure> {
        match &self.path {
            Some(path) if !self.force && path.symlink_metadata().is_ok() => Err(exists(path)),
            _ => Ok(()),
        }
    }
}
