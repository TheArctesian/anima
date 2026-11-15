//! The real `anima-database` program against this computer's PostgreSQL 17.

use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Output, Stdio};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use sqlx::Connection as _;
use sqlx::postgres::PgConnection;
use tempfile::TempDir;

/// The PostgreSQL 17 the gates already need. A test never skips for lack of it (CQ-024).
fn tree() -> PathBuf {
    let named = std::env::var_os("ANIMA_POSTGRES_TREE").map(PathBuf::from);
    let known = [
        "/opt/homebrew/opt/postgresql@17",
        "/usr/local/opt/postgresql@17",
        "/usr/lib/postgresql/17",
    ]
    .map(PathBuf::from);
    named
        .into_iter()
        .chain(known)
        .find(|folder| folder.join("bin/postgres").is_file())
        .expect("no PostgreSQL 17 found: install it or set ANIMA_POSTGRES_TREE")
}

struct Place {
    folder: TempDir,
}

impl Place {
    fn new() -> Self {
        Self {
            folder: tempfile::tempdir().unwrap(),
        }
    }

    fn root(&self) -> PathBuf {
        self.folder.path().join("db root")
    }

    fn command(&self, name: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_anima-database"));
        command
            .arg(name)
            .arg("--tree")
            .arg(tree())
            .arg("--root")
            .arg(self.root())
            .arg("--socket-folder")
            .arg(self.folder.path().join("s"));
        command
    }

    /// Starts `serve` and waits for its `ready` line.
    fn serve(&self) -> Serving {
        let mut child = self
            .command("serve")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert_eq!(line, "ready\n", "serve did not get ready");
        Serving {
            child: Some(child),
            _output: output,
        }
    }

    fn hub_url(&self) -> String {
        fs::read_to_string(self.root().join("urls/hub.url")).unwrap()
    }

    /// One statement as the hub, signed in from its URL file as the hub signs in.
    fn as_hub(&self, statement: &str) -> Result<(), sqlx::Error> {
        let url = self.hub_url();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut session = PgConnection::connect(url.trim()).await?;
                sqlx::raw_sql(statement).execute(&mut session).await?;
                session.close().await
            })
    }

    fn server_runs(&self) -> bool {
        self.root().join("cluster/postmaster.pid").exists()
    }
}

/// A running `serve`. Dropping it asks it to stop, so a failed test leaves no server.
struct Serving {
    child: Option<Child>,
    _output: BufReader<ChildStdout>,
}

impl Serving {
    fn signal(&self, signal: Signal) {
        let id = self.child.as_ref().unwrap().id();
        kill(Pid::from_raw(i32::try_from(id).unwrap()), signal).unwrap();
    }

    fn finish(mut self, signal: Signal) -> Output {
        self.signal(signal);
        self.child.take().unwrap().wait_with_output().unwrap()
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let id = i32::try_from(child.id()).unwrap();
            kill(Pid::from_raw(id), Signal::SIGTERM).ok();
            child.wait().ok();
        }
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn everything_printed(output: &Output) -> String {
    format!("{}{}", text(&output.stdout), text(&output.stderr))
}

#[test]
fn db_002_the_hub_signs_in_from_its_url_file_and_sigterm_stops_the_server() {
    let place = Place::new();
    let serving = place.serve();

    place.as_hub("select 1").unwrap();
    assert!(place.server_runs());

    let output = serving.finish(Signal::SIGTERM);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(!place.server_runs());
    assert!(place.as_hub("select 1").is_err());
}

#[test]
fn a_second_serve_keeps_the_cluster_and_the_logins() {
    let place = Place::new();
    let first = place.serve();
    let url = place.hub_url();
    assert!(first.finish(Signal::SIGINT).status.success());

    let _second = place.serve();

    assert_eq!(place.hub_url(), url);
    place.as_hub("select 1").unwrap();
}

#[test]
fn a_second_copy_ends_with_its_own_exit_code_and_the_first_serves_on() {
    let place = Place::new();
    let _first = place.serve();

    let second = place.command("serve").output().unwrap();

    assert_eq!(second.status.code(), Some(10));
    assert!(text(&second.stderr).contains("another copy already runs this database"));
    place.as_hub("select 1").unwrap();
}

#[test]
fn a_killed_program_leaves_a_server_that_the_next_start_takes_over() {
    let place = Place::new();
    let killed = place.serve();
    let old_server = fs::read_to_string(place.root().join("cluster/postmaster.pid")).unwrap();
    drop(killed.finish(Signal::SIGKILL));
    assert!(
        place.server_runs(),
        "the server outlives its killed program"
    );
    place.as_hub("select 1").unwrap();

    let _next = place.serve();

    let new_server = fs::read_to_string(place.root().join("cluster/postmaster.pid")).unwrap();
    assert_ne!(new_server.lines().next(), old_server.lines().next());
    place.as_hub("select 1").unwrap();
}

#[test]
fn log_007_status_tells_the_versions_and_no_secret() {
    let place = Place::new();
    let before = place.command("status").output().unwrap();
    assert_eq!(
        text(&before.stdout),
        "tree: PostgreSQL 17\ncluster: none\nrunning: no\n"
    );

    let serving = place.serve();
    let during = place.command("status").output().unwrap();
    assert_eq!(
        text(&during.stdout),
        "tree: PostgreSQL 17\ncluster: PostgreSQL 17\nrunning: yes\n"
    );
    let served = serving.finish(Signal::SIGTERM);

    let password = place.hub_url();
    let password = password
        .split(':')
        .nth(2)
        .unwrap()
        .split('@')
        .next()
        .unwrap();
    assert_eq!(password.len(), 48);
    for output in [&before, &during, &served] {
        assert!(!everything_printed(output).contains(password));
    }
    let log = fs::read_to_string(place.root().join("server.log")).unwrap();
    assert!(!log.contains(password));
}

#[test]
fn a_folder_that_is_not_a_tree_is_refused_before_anything_is_made() {
    let place = Place::new();
    let empty = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_anima-database"))
        .args(["serve", "--tree"])
        .arg(empty.path())
        .arg("--root")
        .arg(place.root())
        .arg("--socket-folder")
        .arg(place.folder.path().join("s"))
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("has no program `postgres`"));
    assert!(!Path::new(&place.root()).exists());
}
