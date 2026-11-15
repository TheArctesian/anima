//! A scratch database for the tests: a new cluster in a temporary folder, run by this
//! computer's PostgreSQL 17. The folder name has a space, as `Application Support` has.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;

use crate::cluster::{self, SUPERUSER};
use crate::local_tree::local_tree;
use crate::{Layout, Limits, Lock, Password, Program, Server, Settings, Tree};

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
    pub password: Password,
    pub lock: Lock,
    _folder: TempDir,
}

impl Database {
    /// A new cluster; no server runs yet.
    pub fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let layout = Layout::new(folder.path().join("db root"), folder.path().join("s")).unwrap();
        let tree = local_tree();
        let password = Password::from_bytes(&[9; 24]);
        let lock = crate::lock(&layout).unwrap();
        cluster::create(&lock, &tree, &layout, &password).unwrap();
        Self {
            tree,
            layout,
            password,
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
    pub fn psql(&self, database: &str) -> Command {
        let mut psql = Command::new(self.tree.path_of(Program::Postgres).with_file_name("psql"));
        psql.env_clear()
            .env("PGPASSWORD", self.password.expose())
            .args(["-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1"])
            .arg("-h")
            .arg(self.layout.socket_folder())
            .args(["-U", SUPERUSER, "-d", database]);
        psql
    }

    /// Runs one statement as the superuser in the `postgres` database; `Err` when it fails.
    pub fn sql(&self, statement: &str) -> Result<String, String> {
        let output = self
            .psql("postgres")
            .args(["-c", statement])
            .output()
            .unwrap();
        if output.status.success() {
            Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    pub fn pid_file(&self) -> PathBuf {
        self.layout.cluster().join("postmaster.pid")
    }
}
