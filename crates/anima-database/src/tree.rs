//! One PostgreSQL tree: a folder with `bin/`, as the application bundle carries it and as a
//! system package installs it. The server finds its own `lib/` and `share/` from `bin/`.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
mod tests;

/// The PostgreSQL programs this crate runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    Postgres,
    Initdb,
    PgCtl,
    PgControldata,
    PgDump,
    PgRestore,
    PgAmcheck,
}

impl Program {
    /// The programs a tree must have before the database can start.
    const NEEDED_TO_START: [Self; 3] = [Self::Postgres, Self::Initdb, Self::PgCtl];

    const fn file_name(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Initdb => "initdb",
            Self::PgCtl => "pg_ctl",
            Self::PgControldata => "pg_controldata",
            Self::PgDump => "pg_dump",
            Self::PgRestore => "pg_restore",
            Self::PgAmcheck => "pg_amcheck",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("the PostgreSQL tree has no program `{program}`")]
    ProgramMissing { program: &'static str },
    #[error("`postgres --version` could not run")]
    VersionRun(#[source] std::io::Error),
    #[error("`postgres --version` gave no PostgreSQL version")]
    VersionText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    bin: PathBuf,
}

impl Tree {
    /// The tree in `folder`.
    ///
    /// # Errors
    /// When `folder/bin` lacks a program the database needs to start.
    pub fn at(folder: &Path) -> Result<Self, TreeError> {
        let tree = Self {
            bin: folder.join("bin"),
        };
        for program in Program::NEEDED_TO_START {
            if !tree.path_of(program).is_file() {
                return Err(TreeError::ProgramMissing {
                    program: program.file_name(),
                });
            }
        }
        Ok(tree)
    }

    #[must_use]
    pub fn path_of(&self, program: Program) -> PathBuf {
        self.bin.join(program.file_name())
    }

    /// A command for `program` with a fixed environment: nothing of the caller's environment
    /// reaches PostgreSQL, so a `PGPASSWORD`, a `PGHOST` or a locale of the operator's shell
    /// cannot change what the program does. The server does not start on macOS without a
    /// valid locale.
    #[must_use]
    pub fn command(&self, program: Program) -> Command {
        let mut command = Command::new(self.path_of(program));
        command.env_clear().env("LC_ALL", "C").env("TZ", "UTC");
        command
    }

    /// The major version of the tree's server: 17 for PostgreSQL 17.12.
    ///
    /// # Errors
    /// When `postgres --version` does not run or prints no version.
    pub fn major_version(&self) -> Result<u32, TreeError> {
        let output = self
            .command(Program::Postgres)
            .arg("--version")
            .output()
            .map_err(TreeError::VersionRun)?;
        major_in(&String::from_utf8_lossy(&output.stdout)).ok_or(TreeError::VersionText)
    }
}

/// The major version in the line `postgres --version` prints, for example
/// `postgres (PostgreSQL) 17.12 (Homebrew)` or `postgres (PostgreSQL) 18beta1`.
fn major_in(version_line: &str) -> Option<u32> {
    let after = version_line.split("(PostgreSQL)").nth(1)?.trim_start();
    let digits = after
        .find(|character: char| !character.is_ascii_digit())
        .map_or(after, |end| &after[..end]);
    digits.parse().ok()
}
