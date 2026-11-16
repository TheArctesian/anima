//! The members of the hub's group: which program, which arguments, which environment. Pure.
//!
//! Start order: the database, the one-shot migration, the gateway, the hub. The stop order is
//! the reverse.
//!
//! **Place for S6 (the link):** `anima-link` is one more member. It starts after the hub, with
//! the hub's address from the hub's ready line, and stops first. Add its `Spec` here and one
//! step at the end of `Group::start`.

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
/// This program. `install` copies it with the others.
pub const LOCAL_HUB: &str = "anima-local-hub";

/// Every program the unit needs, in one folder.
pub const PROGRAMS: [&str; 4] = [LOCAL_HUB, HUB, DATABASE, GATEWAY];

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
        ("ANIMA_GUACD", gateway.to_string().into()),
        ("ANIMA_LOG", "info".into()),
        // S5 owns this address and the access rules. S4 gives the hub a loopback address with
        // plain HTTP (NET-003 permits that on loopback only) so that the group can start.
        // ANIMA_PUBLIC_ORIGIN, ANIMA_PERMITTED_NETWORKS and ANIMA_PROBE_DENY are not set
        // here: with the hub's defaults a loopback client is refused, and only `/health`
        // answers.
        ("ANIMA_LISTEN", listen.to_string().into()),
    ]
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
