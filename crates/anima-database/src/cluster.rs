//! The cluster: the PostgreSQL data folder of one bundled database.
//!
//! These choices are fixed when the cluster is made and cannot change without a dump: UTF-8,
//! the built-in `C.UTF-8` locale provider (no ICU, and no index that an operating system
//! update can break) and data checksums (PostgreSQL 18 turns them on, and an upgrade needs
//! both sides equal).
//!
//! Who may sign in is this crate's own `pg_hba.conf`. The superuser has no password: it signs
//! in only as the operating-system user who runs the database, which PostgreSQL asks the
//! kernel about (`peer`). Every other login needs `scram-sha-256`. There is no rule for a
//! network connection, and the server has no network listener (DB-001).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use nix::unistd::{Uid, User};

use crate::{Layout, Lock, Program, Tree, owner_only};

#[cfg(test)]
mod tests;

/// The superuser of a bundled cluster. Only this crate signs in with it, for the roles, the
/// checks and an upgrade; the hub is never given it.
pub const SUPERUSER: &str = "anima_super";

/// The name of the map in `pg_ident.conf` that lets the operating-system user be [`SUPERUSER`].
const USER_MAP: &str = "anima";

const ACCESS_RULES: &str = "\
# Written by anima-database; a change here is lost when the cluster is made again.
# The superuser: only the operating-system user who runs the database, no password.
local all anima_super peer map=anima
# Every other login: its password, never in clear text.
local all all scram-sha-256
";

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("could not {step}")]
    Io {
        step: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("initdb failed: {reason}")]
    Initdb { reason: String },
    #[error("the cluster's PG_VERSION file holds no version number")]
    VersionFile,
    #[error("a cluster folder is there already; a new cluster is never made over it")]
    AlreadyThere,
    #[error("the operating system has no name for the user who runs the database")]
    UserName,
    #[error(
        "the name of the user who runs the database starts with `/` or has a control \
         character; PostgreSQL would read it as a pattern or as two lines"
    )]
    UserNameForm,
    #[error("pg_controldata did not say how the cluster was stopped")]
    ControlText,
}

fn io_error(step: &'static str) -> impl FnOnce(io::Error) -> ClusterError {
    move |source| ClusterError::Io { step, source }
}

/// How a cluster's major version stands to the tree that would run it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// The tree can run the cluster.
    Same,
    /// The cluster needs an upgrade before this tree can run it.
    ClusterOlder { cluster: u32, tree: u32 },
    /// A newer application made or upgraded the cluster. This tree must never touch it.
    ClusterNewer { cluster: u32, tree: u32 },
}

#[must_use]
pub fn fit(cluster: u32, tree: u32) -> Fit {
    match cluster.cmp(&tree) {
        std::cmp::Ordering::Equal => Fit::Same,
        std::cmp::Ordering::Less => Fit::ClusterOlder { cluster, tree },
        std::cmp::Ordering::Greater => Fit::ClusterNewer { cluster, tree },
    }
}

/// The major version in the cluster's `PG_VERSION`, or `None` when no cluster is there yet.
///
/// # Errors
/// When the file cannot be read or holds no number.
pub fn major_version(layout: &Layout) -> Result<Option<u32>, ClusterError> {
    match fs::read_to_string(layout.cluster().join("PG_VERSION")) {
        Ok(text) => text
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| ClusterError::VersionFile),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io_error("read the cluster's PG_VERSION")(source)),
    }
}

/// Makes the cluster with `initdb`. The caller holds the [`Lock`]: two copies that make one
/// cluster remove each other's half-made folder.
///
/// The cluster is made beside its place, gets its access rules, and is moved in last. So a
/// crash at any point leaves either no cluster, and the next start makes one, or a whole
/// cluster. No password exists at this point: the superuser has none.
///
/// # Errors
/// When a cluster folder is there already, a folder or a file cannot be made, or `initdb`
/// fails.
pub fn create(_lock: &Lock, tree: &Tree, layout: &Layout) -> Result<(), ClusterError> {
    if layout.cluster().exists() {
        return Err(ClusterError::AlreadyThere);
    }
    owner_only::folder(layout.root()).map_err(io_error("make the database root"))?;
    let being_made = layout.cluster_being_made();
    if let Err(error) = fs::remove_dir_all(&being_made)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(io_error("remove a half-made cluster")(error));
    }
    run_initdb(tree, &being_made)?;
    replace(&being_made.join("pg_hba.conf"), ACCESS_RULES)
        .map_err(io_error("write the cluster's access rules"))?;
    replace(&being_made.join("pg_ident.conf"), &user_map(&this_user()?))
        .map_err(io_error("write the cluster's user map"))?;
    fs::rename(&being_made, layout.cluster()).map_err(io_error("move the new cluster into place"))
}

/// Lets the operating-system user who runs this program be the cluster's superuser, and
/// nobody else. Run before each start: the user's name can change between two starts.
///
/// # Errors
/// When the user has no name or the map cannot be written.
pub fn admit_this_user(_lock: &Lock, layout: &Layout) -> Result<(), ClusterError> {
    replace(
        &layout.cluster().join("pg_ident.conf"),
        &user_map(&this_user()?),
    )
    .map_err(io_error("write the cluster's user map"))
}

fn this_user() -> Result<String, ClusterError> {
    let name = User::from_uid(Uid::effective())
        .ok()
        .flatten()
        .map(|user| user.name)
        .ok_or(ClusterError::UserName)?;
    if is_plain_user_name(&name) {
        Ok(name)
    } else {
        Err(ClusterError::UserNameForm)
    }
}

/// Whether `name` means only itself in `pg_ident.conf`. A leading `/` makes a quoted value a
/// regular expression there, and a line break would start a second mapping.
fn is_plain_user_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('/') && !name.chars().any(char::is_control)
}

/// The `pg_ident.conf` text that maps `user` to [`SUPERUSER`]. The name is quoted, so no
/// character in it can add a second mapping.
fn user_map(user: &str) -> String {
    format!(
        "# Written by anima-database at each start.\n{USER_MAP} \"{}\" {SUPERUSER}\n",
        user.replace('"', "\"\"")
    )
}

/// Writes `text` beside `file` and moves it in, so the server never reads half a file.
fn replace(file: &Path, text: &str) -> io::Result<()> {
    let mut beside = file.to_path_buf().into_os_string();
    beside.push(".new");
    let mut new = owner_only::new_file(beside.as_ref())?;
    new.write_all(text.as_bytes())?;
    new.sync_all()?;
    fs::rename(&beside, file)
}

/// Whether the cluster's last server wrote its shutdown checkpoint. `false` after a kill or
/// a power cut: the next start recovers from the write-ahead log.
///
/// # Errors
/// When `pg_controldata` cannot run or prints no state.
pub fn was_stopped_cleanly(tree: &Tree, layout: &Layout) -> Result<bool, ClusterError> {
    let output = tree
        .command(Program::PgControldata)
        .arg(layout.cluster())
        .output()
        .map_err(io_error("run pg_controldata"))?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Database cluster state:"))
        .map(|state| state.trim() == "shut down")
        .ok_or(ClusterError::ControlText)
}

fn run_initdb(tree: &Tree, cluster: &Path) -> Result<(), ClusterError> {
    let output = tree
        .command(Program::Initdb)
        .arg("--pgdata")
        .arg(cluster)
        .args(["--username", SUPERUSER])
        // Replaced by this crate's own rules before any server starts.
        .args(["--auth-local", "peer", "--auth-host", "reject"])
        .args(["--encoding", "UTF8", "--locale", "C"])
        .args([
            "--locale-provider",
            "builtin",
            "--builtin-locale",
            "C.UTF-8",
        ])
        .arg("--data-checksums")
        .output()
        .map_err(io_error("run initdb"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(ClusterError::Initdb {
        reason: initdb_reason(&String::from_utf8_lossy(&output.stderr)),
    })
}

/// The cause in what `initdb` printed: its own `initdb: error:` line, or the `FATAL:` line of
/// the server it runs to fill the cluster (for example "could not create shared memory
/// segment: No space left on device"). No password exists when `initdb` runs.
fn initdb_reason(errors: &str) -> String {
    let own = errors
        .lines()
        .find_map(|line| line.strip_prefix("initdb: error: "));
    let of_server = errors
        .lines()
        .find(|line| line.contains("FATAL:"))
        .map(str::trim);
    own.or(of_server)
        .unwrap_or("it printed no cause")
        .to_owned()
}
