//! A scratch database for the tests: a new cluster in a temporary folder, run by this
//! computer's PostgreSQL 17. The folder name has a space, as `Application Support` has.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;

use crate::cluster::{self, SUPERUSER};
use crate::{Layout, Limits, Lock, Program, Server, Settings, Tree};

/// Where a package puts PostgreSQL 17.
const KNOWN_TREES: [&str; 3] = [
    "/opt/homebrew/opt/postgresql@17",
    "/usr/local/opt/postgresql@17",
    "/usr/lib/postgresql/17",
];

pub const SETTINGS: Settings = Settings {
    max_connections: 10,
    shared_buffers_mb: 16,
    flush_through_drive_cache: false,
};
pub const LIMITS: Limits = Limits {
    ready: Duration::from_secs(60),
    stop: Duration::from_secs(30),
};

pub struct Database {
    pub tree: Tree,
    pub layout: Layout,
    pub lock: Lock,
    _folder: TempDir,
}

impl Database {
    /// A new cluster; no server runs yet.
    pub fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let layout = Layout::new(folder.path().join("db root"), folder.path().join("s")).unwrap();
        let tree = local_tree();
        let lock = crate::lock(&layout).unwrap();
        cluster::create(&lock, &tree, &layout).unwrap();
        Self {
            tree,
            layout,
            lock,
            _folder: folder,
        }
    }

    pub fn start(&self) -> Server {
        let mut server = Server::start(&self.lock, &self.tree, &self.layout, SETTINGS).unwrap();
        server.wait_until_ready(&self.layout, LIMITS).unwrap();
        server
    }

    /// The tree's `psql`, signed in as the superuser on `database`, with no start-up file.
    /// No password: the test runs as the operating-system user that the cluster admits.
    pub fn psql(&self, database: &str) -> Command {
        let mut psql = Command::new(self.psql_program());
        psql.env_clear()
            .args(["-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1"])
            .arg("-h")
            .arg(self.layout.socket_folder())
            .args(["-U", SUPERUSER, "-d", database]);
        psql
    }

    /// Runs one statement as the superuser in the `postgres` database; `Err` when it fails.
    pub fn sql(&self, statement: &str) -> Result<String, String> {
        self.sql_in("postgres", statement)
    }

    /// Runs statements as the superuser in `database`; the answer of the last one.
    pub fn sql_in(&self, database: &str, statements: &str) -> Result<String, String> {
        let output = self
            .psql(database)
            .args(["-c", statements])
            .output()
            .unwrap();
        if output.status.success() {
            let printed = String::from_utf8(output.stdout).unwrap();
            Ok(printed.trim().lines().last().unwrap_or_default().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    /// One statement as `login` with `password`, as a client with a password signs in.
    pub fn sql_as(&self, login: &str, password: &str, statement: &str) -> Result<String, String> {
        let output = Command::new(self.psql_program())
            .env_clear()
            .env("PGPASSWORD", password)
            .args(["-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1", "-h"])
            .arg(self.layout.socket_folder())
            .args(["-U", login, "-d", "postgres", "-c", statement])
            .output()
            .unwrap();
        if output.status.success() {
            Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    /// `psql` is a tool of the tests only. The application's own tree does not carry it, so
    /// with such a tree in `ANIMA_POSTGRES_TREE` the tests take `psql` from an installed one.
    fn psql_program(&self) -> PathBuf {
        let own = self.tree.path_of(Program::Postgres).with_file_name("psql");
        if own.is_file() {
            return own;
        }
        KNOWN_TREES
            .iter()
            .map(|folder| PathBuf::from(folder).join("bin/psql"))
            .find(|psql| psql.is_file())
            .expect("no psql found in an installed PostgreSQL 17")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.layout.cluster().join("postmaster.pid")
    }
}

/// # Panics
/// When this computer has no PostgreSQL 17.
pub fn local_tree() -> Tree {
    let named = std::env::var_os("ANIMA_POSTGRES_TREE").map(PathBuf::from);
    let known = KNOWN_TREES.map(PathBuf::from);
    named
        .iter()
        .chain(&known)
        .find_map(|folder| Tree::at(folder).ok())
        .expect(
            "no PostgreSQL 17 found: install it (macOS: brew install postgresql@17) \
             or set ANIMA_POSTGRES_TREE to a folder with bin/postgres",
        )
}
