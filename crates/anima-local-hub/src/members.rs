//! The members of the hub's group: which program, which arguments, which environment. Pure.
//!
//! Start order: the database, the one-shot migration, the gateway, the hub, the link. The stop
//! order is the reverse. The link is the one member that the group starts again alone (F-6).

use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anima_database::Role;

use crate::Folders;
use crate::child::Spec;

#[cfg(test)]
mod tests;

pub const DATABASE: &str = "anima-database";
pub const GATEWAY: &str = "anima-gateway";
pub const HUB: &str = "anima-hub";
/// The link to the devices (S1, S5). A Go program.
pub const LINK: &str = "anima-link";
/// This program. `install` copies it with the others.
pub const LOCAL_HUB: &str = "anima-local-hub";

/// Every program the unit needs, in one folder.
pub const PROGRAMS: [&str; 5] = [LOCAL_HUB, HUB, DATABASE, GATEWAY, LINK];

/// The most bytes of a Unix socket path: macOS has 104 with the end byte. The link checks the
/// same limit, but only after it reached its relay.
pub const SOCKET_PATH_LIMIT: usize = 103;

/// The folder of the PostgreSQL tree beside the folder of the programs.
const TREE: &str = "database-tree";

/// The programs and the PostgreSQL tree that this run uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    programs: PathBuf,
    tree: PathBuf,
}

impl Carried {
    #[must_use]
    pub const fn new(programs: PathBuf, tree: PathBuf) -> Self {
        Self { programs, tree }
    }

    /// The copy that `install` made below `folder`: `bin/` and `database-tree/`.
    #[must_use]
    pub fn installed_in(folder: &Path) -> Self {
        Self::new(folder.join("bin"), folder.join(TREE))
    }

    /// The programs beside `program`, which is in `<folder>/bin/`, and the tree in `<folder>`.
    /// `None` when `program` has no such folders above it.
    #[must_use]
    pub fn around(program: &Path) -> Option<Self> {
        Some(Self::installed_in(program.parent()?.parent()?))
    }

    #[must_use]
    pub fn programs(&self) -> &Path {
        &self.programs
    }

    #[must_use]
    pub fn tree(&self) -> &Path {
        &self.tree
    }

    #[must_use]
    pub fn program(&self, name: &str) -> PathBuf {
        self.programs.join(name)
    }
}

fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// `anima-database serve` with its folders named, so that it reads nothing of the environment.
#[must_use]
pub fn database(carried: &Carried, folders: &Folders) -> Spec {
    let mut args = words(&["serve", "--tree"]);
    args.push(carried.tree().into());
    args.push("--root".into());
    args.push(folders.database().root().into());
    args.push("--socket-folder".into());
    args.push(folders.database().socket_folder().into());
    Spec {
        name: DATABASE,
        program: carried.program(DATABASE),
        args,
        env: Vec::new(),
    }
}

/// `anima-hub migrate` as the owner login (R-016).
#[must_use]
pub fn migration(carried: &Carried, folders: &Folders) -> Spec {
    Spec {
        name: "anima-hub migrate",
        program: carried.program(HUB),
        args: words(&["migrate"]),
        env: vec![(
            "ANIMA_DATABASE_URL_FILE",
            folders.database().url_file(Role::Owner).into(),
        )],
    }
}

/// The gateway on a loopback port that the operating system picks; its ready line names it.
#[must_use]
pub fn gateway(carried: &Carried) -> Spec {
    Spec {
        name: GATEWAY,
        program: carried.program(GATEWAY),
        args: words(&["listen", "127.0.0.1:0"]),
        env: Vec::new(),
    }
}

/// The hub as the runtime login.
#[must_use]
pub fn hub(carried: &Carried, folders: &Folders, listen: SocketAddr, gateway: SocketAddr) -> Spec {
    Spec {
        name: HUB,
        program: carried.program(HUB),
        args: Vec::new(),
        env: hub_environment(folders, listen, gateway),
    }
}

/// A socket path of the link that is too long for the operating system.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "anima-link cannot start: its socket {} is {length} bytes and the limit is \
     {SOCKET_PATH_LIMIT}; use a hub folder with a shorter path",
    path.display()
)]
pub struct LongSocketPath {
    pub path: PathBuf,
    pub length: usize,
}

/// `anima-link hub` with its state below the hub root and the hub's socket as its target. It
/// needs nothing of the environment.
///
/// # Errors
/// [`LongSocketPath`] when a socket that the link or the hub makes cannot have its path.
pub fn link(carried: &Carried, folders: &Folders) -> Result<Spec, LongSocketPath> {
    for path in [folders.link_control(), folders.link_socket()] {
        let length = path.as_os_str().len();
        if length > SOCKET_PATH_LIMIT {
            return Err(LongSocketPath { path, length });
        }
    }
    let mut args = words(&["hub", "-state"]);
    args.push(folders.link().into());
    args.push("-target".into());
    args.push(folders.link_socket().into());
    Ok(Spec {
        name: LINK,
        program: carried.program(LINK),
        args,
        env: Vec::new(),
    })
}

/// The hub of `spec` as the pool runs it. With the file of the cloud's key the hub is in the
/// hosted mode: it makes device invites and claims for the Anima.ing cloud and refuses every
/// password sign-in (R-129, HMD-038, HMD-039). With a public origin and a web root it names
/// the address of the gateway and serves the web client (R-239). With none of them the hub
/// stays a local hub.
#[must_use]
pub fn hosted(mut spec: Spec, pool: &Pool) -> Spec {
    if let Some(file) = &pool.cloud_key_file {
        spec.env.push(("ANIMA_HOSTED_CLOUD_KEY_FILE", file.into()));
    }
    if let Some(origin) = &pool.public_origin {
        spec.env.push(("ANIMA_PUBLIC_ORIGIN", origin.into()));
    }
    if let Some(root) = &pool.web_root {
        spec.env.push(("ANIMA_WEB_ROOT", root.into()));
    }
    spec
}

/// What only a hub of the pool is told, each by an option of `run`; all absent on the desktop,
/// where the hub keeps its defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pool {
    /// The file of the Anima.ing cloud's key (R-129): the hub runs in the hosted mode.
    pub cloud_key_file: Option<PathBuf>,
    /// The address at which a browser reaches the hub through the gateway (HMD-069). The hub
    /// checks its form and builds the address of Connect GitHub from it.
    pub public_origin: Option<String>,
    /// The folder of the web client that the hub serves (HMD-059).
    pub web_root: Option<PathBuf>,
}

/// Everything the hub is told. A setting that is not here has the hub's default (K7).
fn hub_environment(
    folders: &Folders,
    listen: SocketAddr,
    gateway: SocketAddr,
) -> Vec<(&'static str, OsString)> {
    vec![
        (
            "ANIMA_DATABASE_URL_FILE",
            folders.database().url_file(Role::Hub).into(),
        ),
        ("ANIMA_KEY_FILE", folders.vault_key().into()),
        // The data keys and the file pieces, beside the key (K-E5 Section 1, K-E4 Section 9).
        ("ANIMA_DATA_KEY_DIR", folders.data_keys().into()),
        ("ANIMA_FILES_DIR", folders.files().into()),
        ("ANIMA_GUACD", gateway.to_string().into()),
        ("ANIMA_LOG", "info".into()),
        // The link mode (S5): the hub listens on loopback with plain HTTP (NET-003) and on the
        // link's socket. It permits this computer and the devices of the link (NET-001), so
        // ANIMA_PERMITTED_NETWORKS must not be set. ANIMA_PUBLIC_ORIGIN and ANIMA_PROBE_DENY
        // keep the hub's defaults.
        ("ANIMA_LISTEN", listen.to_string().into()),
        ("ANIMA_LINK_SOCKET", folders.link_socket().into()),
        // The link's control socket: invites and the device list (S6 Section 2.5).
        ("ANIMA_LINK_CONTROL", folders.link_control().into()),
    ]
}

/// Whether `line` is the link's ready line: slog text with `msg="the hub link is ready"`.
#[must_use]
pub fn link_is_ready(line: &str) -> Option<()> {
    let message = line.split_once(" msg=")?.1;
    message
        .starts_with("\"the hub link is ready\"")
        .then_some(())
}

/// The address in the gateway's ready line.
#[must_use]
pub fn gateway_address(line: &str) -> Option<SocketAddr> {
    line.strip_prefix("anima-gateway: listening on ")?
        .parse()
        .ok()
}

/// The address in the hub's `hub started` log line, which is one JSON object.
#[must_use]
pub fn hub_address(line: &str) -> Option<SocketAddr> {
    let line: serde_json::Value = serde_json::from_str(line).ok()?;
    let fields = line.get("fields")?;
    (fields.get("message")?.as_str()? == "hub started")
        .then(|| fields.get("listen")?.as_str()?.parse().ok())?
}
