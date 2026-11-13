//! Runs the `anima` program with a clean environment and its own configuration folder.

pub mod fake_hub;
pub mod tunnel_hub;

use std::path::PathBuf;
use std::process::Stdio;

use tempfile::TempDir;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

/// What one run printed and how it ended.
#[derive(Debug)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// One operator's computer: a home folder, and the variables every run gets.
pub struct Anima {
    home: TempDir,
    variables: Vec<(String, String)>,
}

impl Anima {
    pub fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("a home folder"),
            variables: Vec::new(),
        }
    }

    /// Adds `name=value` to every later run.
    pub fn with(mut self, name: &str, value: &str) -> Self {
        self.variables.push((name.to_owned(), value.to_owned()));
        self
    }

    /// `$XDG_CONFIG_HOME/anima`.
    pub fn config_dir(&self) -> PathBuf {
        self.home.path().join("config").join("anima")
    }

    pub async fn run(&self, args: &[&str]) -> Outcome {
        self.run_with_input(args, "").await
    }

    /// Runs with `input` on standard input, which is a pipe, never a terminal.
    pub async fn run_with_input(&self, args: &[&str], input: &str) -> Outcome {
        let mut command = Command::new(env!("CARGO_BIN_EXE_anima"));
        command
            .args(args)
            .env_clear()
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join("config"))
            .env("ANIMA_TOKEN_STORE", "file")
            .envs(self.variables.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("anima starts");
        let mut stdin = child.stdin.take().expect("a stdin pipe");
        stdin
            .write_all(input.as_bytes())
            .await
            .expect("input written");
        drop(stdin);
        let output = child.wait_with_output().await.expect("anima ends");
        Outcome {
            code: output.status.code().expect("an exit code, not a signal"),
            stdout: String::from_utf8(output.stdout).expect("UTF-8 output"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8 errors"),
        }
    }
}

impl Outcome {
    /// Standard output as JSON.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).expect("standard output is JSON")
    }

    /// Standard error as the CMD-022 JSON error.
    pub fn error(&self) -> serde_json::Value {
        let value: serde_json::Value =
            serde_json::from_str(self.stderr.trim()).expect("standard error is JSON");
        value["error"].clone()
    }

    /// Fails when `secret` is anywhere in what the run printed (CMD-014).
    pub fn assert_never_shows(&self, secret: &str) {
        assert!(!self.stdout.contains(secret), "token on stdout: {self:?}");
        assert!(!self.stderr.contains(secret), "token on stderr: {self:?}");
    }
}
