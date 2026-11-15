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
        self.as_login("hub", statement)
    }

    /// Statements as the login of `urls/<name>.url`.
    fn as_login(&self, name: &str, statements: &str) -> Result<(), sqlx::Error> {
        let url = fs::read_to_string(self.root().join(format!("urls/{name}.url"))).unwrap();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut session = PgConnection::connect(url.trim()).await?;
                sqlx::raw_sql(statements).execute(&mut session).await?;
                session.close().await
            })
    }

    fn server_runs(&self) -> bool {
        self.root().join("cluster/postmaster.pid").exists()
    }
}

/// A failed test must leave no server: one that a killed `serve` left is stopped at once.
impl Drop for Place {
    fn drop(&mut self) {
        if self.server_runs() {
            Command::new(tree().join("bin/pg_ctl"))
                .arg("stop")
                .arg("--pgdata")
                .arg(self.root().join("cluster"))
                .args(["--mode", "immediate", "--silent"])
                .status()
                .ok();
        }
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
        "tree: PostgreSQL 17\ncluster: none\nserver: not running\n"
    );

    let serving = place.serve();
    let during = place.command("status").output().unwrap();
    assert_eq!(
        text(&during.stdout),
        "tree: PostgreSQL 17\ncluster: PostgreSQL 17\nserver: accepts connections\n"
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

#[test]
fn a_cluster_of_another_major_version_is_refused_and_not_touched() {
    let place = Place::new();
    assert!(place.serve().finish(Signal::SIGTERM).status.success());
    let version_file = place.root().join("cluster/PG_VERSION");

    for (version, told) in [("18", "newer Anima"), ("16", "upgrades the database")] {
        fs::write(&version_file, format!("{version}\n")).unwrap();
        let before = files_of(&place.root().join("cluster"));

        let refused = place.command("serve").output().unwrap();

        assert_eq!(refused.status.code(), Some(11), "{version}");
        assert!(
            text(&refused.stderr).contains(told),
            "{}",
            text(&refused.stderr)
        );
        assert_eq!(files_of(&place.root().join("cluster")), before, "{version}");
    }

    fs::write(&version_file, "17\n").unwrap();
    let _serving = place.serve();
    place.as_hub("select 1").unwrap();
}

/// Every file below `folder` with its bytes.
fn files_of(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_of(&path));
        } else {
            let bytes = fs::read(&path).unwrap();
            files.push((path, bytes));
        }
    }
    files.sort();
    files
}

#[test]
fn db_007_verify_backup_and_restore_carry_the_owners_rows_to_a_new_database() {
    let source = Place::new();
    let _serving = source.serve();
    source
        .as_login(
            "owner",
            "create schema devices; create table devices.devices (id int primary key); \
             insert into devices.devices select generate_series(1, 300); \
             grant usage on schema devices to anima_runtime; \
             grant select on devices.devices to anima_runtime",
        )
        .unwrap();
    let file = source.folder.path().join("anima.backup");

    let checked = source.command("verify").output().unwrap();
    assert_eq!(
        text(&checked.stdout),
        "sound\n",
        "{}",
        text(&checked.stderr)
    );
    let saved = source.command("backup").arg(&file).output().unwrap();
    assert!(saved.status.success(), "{}", text(&saved.stderr));

    let target = Place::new();
    let _target_serving = target.serve();
    let restored = target.command("restore").arg(&file).output().unwrap();
    assert!(restored.status.success(), "{}", text(&restored.stderr));
    target
        .as_hub("do $$ begin assert (select count(*) from devices.devices) = 300; end $$")
        .unwrap();

    let again = target.command("restore").arg(&file).output().unwrap();
    assert_eq!(
        again.status.code(),
        Some(1),
        "a second restore finds the rows there"
    );
    assert!(text(&again.stderr).contains("changed nothing"));
}
