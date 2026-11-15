//! Each test makes its own cluster and runs a real server from this computer's PostgreSQL 17.

use std::fs;
use std::mem;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use tempfile::TempDir;

use super::pid_file::PidFile;
use super::{Limits, Server, ServerError, Settings, Stopped, lock, stop_left_over};
use crate::cluster::{self, SUPERUSER};
use crate::local_tree::local_tree;
use crate::{Layout, Password, Program, Tree};

const SETTINGS: Settings = Settings {
    max_connections: 10,
    shared_buffers_mb: 16,
    flush_through_drive_cache: false,
};
const LIMITS: Limits = Limits {
    ready: Duration::from_secs(60),
    stop: Duration::from_secs(30),
};

struct Database {
    tree: Tree,
    layout: Layout,
    password: Password,
    _folder: TempDir,
}

impl Database {
    /// A new cluster in a temporary folder.
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let layout = Layout::new(folder.path().join("db root"), folder.path().join("s")).unwrap();
        let tree = local_tree();
        let password = Password::from_bytes(&[9; 24]);
        cluster::create(&tree, &layout, &password).unwrap();
        Self {
            tree,
            layout,
            password,
            _folder: folder,
        }
    }

    fn start(&self) -> Server {
        let mut server = Server::start(&self.tree, &self.layout, SETTINGS).unwrap();
        server.wait_until_ready(&self.layout, LIMITS).unwrap();
        server
    }

    /// Runs one statement as the superuser with the tree's `psql`; `Err` when it fails.
    fn sql(&self, statement: &str) -> Result<String, String> {
        let psql = self.tree.path_of(Program::Postgres).with_file_name("psql");
        let output = Command::new(psql)
            .env_clear()
            .env("PGPASSWORD", self.password.expose())
            .args(["-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1"])
            .arg("-h")
            .arg(self.layout.socket_folder())
            .args(["-U", SUPERUSER, "-d", "postgres", "-c", statement])
            .output()
            .unwrap();
        if output.status.success() {
            Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    fn pid_file(&self) -> std::path::PathBuf {
        self.layout.cluster().join("postmaster.pid")
    }
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn pid_file_text(process_id: u32, started_at: u64, status: &str) -> String {
    format!("{process_id}\n/cluster\n{started_at}\n5432\n/s\n\n  5432001    65536\n{status}\n")
}

#[test]
fn db_001_a_ready_server_listens_on_the_private_socket_only() {
    let database = Database::new();

    let server = database.start();

    assert_eq!(database.sql("show listen_addresses").unwrap(), "");
    assert!(database.layout.socket_file().exists());
    assert_eq!(mode_of(database.layout.socket_folder()), 0o700);
    assert_eq!(mode_of(&database.layout.server_log()), 0o600);
    assert_eq!(server.stop(LIMITS).unwrap(), Stopped::Clean);
    assert!(!database.pid_file().exists());
    assert!(database.sql("select 1").is_err());
}

#[test]
fn db_003_the_superuser_signs_in_with_the_password_it_was_made_with_and_no_other() {
    let mut database = Database::new();
    let _server = database.start();
    assert_eq!(database.sql("select current_user").unwrap(), SUPERUSER);

    database.password = Password::from_bytes(&[8; 24]);

    let refusal = database.sql("select 1").unwrap_err();
    assert!(
        refusal.contains("password authentication failed"),
        "{refusal}"
    );
}

#[test]
fn log_007_the_server_logs_no_statement_also_when_one_fails() {
    let database = Database::new();
    let _server = database.start();

    database
        .sql("create role probe password 'a-secret-in-a-statement'")
        .unwrap();
    database
        .sql("alter role nobody_here password 'a-secret-in-a-failed-statement'")
        .unwrap_err();

    let log = fs::read_to_string(database.layout.server_log()).unwrap();
    assert!(log.contains("nobody_here"), "the failure itself is logged");
    assert!(!log.contains("a-secret"), "{log}");
}

#[test]
fn the_cluster_has_the_choices_that_cannot_change_later() {
    let database = Database::new();
    let _server = database.start();

    let choices = database.sql(
        "select pg_encoding_to_char(encoding) || ' ' || datlocprovider::text || ' ' || datlocale \
         from pg_database where datname = 'postgres'",
    );

    assert_eq!(choices.unwrap(), "UTF8 b C.UTF-8");
    assert_eq!(database.sql("show data_checksums").unwrap(), "on");
}

#[test]
fn the_drive_cache_setting_reaches_the_server() {
    let database = Database::new();
    let settings = Settings {
        flush_through_drive_cache: true,
        ..SETTINGS
    };
    // Linux has no fsync_writethrough; there the server refuses the setting and does not start.
    if cfg!(target_os = "macos") {
        let mut server = Server::start(&database.tree, &database.layout, settings).unwrap();
        server.wait_until_ready(&database.layout, LIMITS).unwrap();

        assert_eq!(
            database.sql("show wal_sync_method").unwrap(),
            "fsync_writethrough"
        );
    }
    let _server = database.start();
    assert_ne!(
        database.sql("show wal_sync_method").unwrap(),
        "fsync_writethrough"
    );
}

#[test]
fn a_second_copy_is_refused_while_the_first_holds_the_lock() {
    let folder = tempfile::tempdir().unwrap();
    let layout = Layout::new(folder.path().join("db"), folder.path().join("s")).unwrap();

    let first = lock(&layout).unwrap();

    assert!(matches!(lock(&layout), Err(ServerError::AnotherCopy)));
    drop(first);
    assert!(lock(&layout).is_ok());
}

#[test]
fn every_acknowledged_row_survives_a_killed_server() {
    let database = Database::new();
    let mut server = database.start();
    database
        .sql("create table marks (n int primary key)")
        .unwrap();

    let killer = server.process_id();
    let acknowledged = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut count = 0;
            while database
                .sql(&format!("insert into marks values ({count})"))
                .is_ok()
            {
                count += 1;
            }
            count
        });
        std::thread::sleep(Duration::from_millis(700));
        kill(
            Pid::from_raw(i32::try_from(killer).unwrap()),
            Signal::SIGKILL,
        )
        .unwrap();
        writer.join().unwrap()
    });
    while server.ended().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        acknowledged > 0,
        "the writer committed nothing before the kill"
    );
    assert!(
        database.pid_file().exists(),
        "a killed server leaves its file"
    );

    assert!(!stop_left_over(&database.tree, &database.layout, LIMITS).unwrap());
    let _server = database.start();

    // The last insert can be committed and the kill can take its answer.
    let rows: i64 = database
        .sql("select count(*) from marks")
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        rows == acknowledged || rows == acknowledged + 1,
        "{rows} rows for {acknowledged} acknowledged"
    );
}

#[test]
fn a_server_left_by_a_killed_copy_is_stopped_before_the_next_start() {
    let database = Database::new();
    let left = database.start();
    let left_process = left.process_id();
    // The copy that owned the server is gone and nothing stops its server.
    mem::forget(left);

    assert!(stop_left_over(&database.tree, &database.layout, LIMITS).unwrap());

    assert!(!database.pid_file().exists());
    let server = database.start();
    assert_ne!(server.process_id(), left_process);
    assert_eq!(database.sql("select 1").unwrap(), "1");
}

#[test]
fn a_child_that_fails_is_not_ready_because_another_server_answers() {
    let database = Database::new();
    let _running = database.start();

    let mut second = Server::start(&database.tree, &database.layout, SETTINGS).unwrap();
    let outcome = second.wait_until_ready(&database.layout, LIMITS);

    assert!(
        matches!(outcome, Err(ServerError::EndedDuringStart { .. })),
        "{outcome:?}"
    );
    assert_eq!(database.sql("select 1").unwrap(), "1");
}

#[test]
fn a_stale_file_that_names_another_program_gets_no_signal_and_blocks_no_start() {
    let database = Database::new();
    // A live process of this user that is not PostgreSQL, with the number a dead server left.
    let mut bystander = Command::new("sleep")
        .arg("600")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    fs::write(
        database.pid_file(),
        pid_file_text(bystander.id(), 1, "ready"),
    )
    .unwrap();

    let was_there = stop_left_over(&database.tree, &database.layout, LIMITS).unwrap();

    assert!(!was_there);
    assert!(
        bystander.try_wait().unwrap().is_none(),
        "the bystander was signalled"
    );
    let _server = database.start();
    assert_eq!(database.sql("select 1").unwrap(), "1");
    bystander.kill().unwrap();
    bystander.wait().unwrap();
}

#[test]
fn a_dropped_server_does_not_keep_running() {
    let database = Database::new();

    drop(database.start());

    assert!(!database.pid_file().exists());
    assert!(database.sql("select 1").is_err());
}

#[test]
fn a_pid_file_is_ready_only_for_the_child_that_wrote_it() {
    let now = SystemTime::now();
    let at = super::epoch_seconds(now);
    let ready = |text: &str, process_id| {
        PidFile::parse(text).is_some_and(|file| file.is_ready_for(process_id, now))
    };

    assert!(ready(&pid_file_text(41, at, "ready"), 41));
    assert!(ready(&pid_file_text(41, at + 2, "ready   "), 41));
    assert!(!ready(&pid_file_text(41, at, "starting"), 41));
    assert!(!ready(&pid_file_text(41, at, "stopping"), 41));
    assert!(
        !ready(&pid_file_text(40, at, "ready"), 41),
        "another server"
    );
    assert!(
        !ready(&pid_file_text(41, at - 1, "ready"), 41),
        "a file from before the start, with a number that came round again"
    );
}

#[test]
fn a_pid_file_that_is_not_whole_yet_says_nothing() {
    assert_eq!(PidFile::parse(""), None);
    assert_eq!(PidFile::parse("41\n/cluster\n"), None);
    assert_eq!(
        PidFile::parse("41\n/cluster\n1794000000\n5432\n/s\n\n  1 2\n"),
        None
    );
    assert_eq!(
        PidFile::parse(&pid_file_text(41, 5, "ready").replace("41", "x")),
        None
    );
}
