//! The tools a builder needs before it claims a job (BLD-028): git, bun, Rust, PostgreSQL 17
//! (R-009), the Tauri system packages, the updater key, the app's own updater public key and, on
//! macOS, the Developer ID signing identity.
//! Pure: whether a path exists is a parameter, and nothing is run.

pub mod readings;

use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;

use super::{environment::Environment, job::Target};

/// One tool the Settings row names when it is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    Git,
    Bun,
    Rust,
    Postgres,
    TauriPackages,
    UpdaterKey,
    /// The key is stored but not unlocked in this app session (D-19).
    UpdaterKeyLocked,
    /// This app was built without the updater public key (`plugins.updater.pubkey` is the
    /// placeholder), so what it builds could never update (R-108).
    UpdaterPublicKey,
    /// The login keychain has no valid identity named `bundle.macOS.signingIdentity`, so this
    /// Mac cannot sign a release (BLD-083).
    SigningIdentity,
}

/// The length of a minisign public key: `Ed`, the 8-byte key ID and the 32-byte key.
const PUBLIC_KEY_BYTES: usize = 42;

/// [`Tool::UpdaterPublicKey`] unless `line`, the key line of the app's updater public key, is a
/// minisign public key (R-108). The placeholder has no key line.
pub fn app_public_key(line: Option<&str>) -> Option<Tool> {
    let key = line
        .and_then(|line| STANDARD.decode(line).ok())
        .is_some_and(|bytes| bytes.len() == PUBLIC_KEY_BYTES && bytes.starts_with(b"Ed"));
    (!key).then_some(Tool::UpdaterPublicKey)
}

/// [`Tool::SigningIdentity`] unless `keychain`, the lines `security find-identity -v -p
/// codesigning` printed, lists `identity`, the one this app's `bundle.macOS.signingIdentity`
/// names (BLD-083). `-v` lists valid identities only, each name in double quotes.
pub fn signing_identity(identity: Option<&str>, keychain: &[String]) -> Option<Tool> {
    let found = identity
        .filter(|name| !name.is_empty())
        .map(|name| format!("\"{name}\""))
        .is_some_and(|quoted| keychain.iter().any(|line| line.contains(&quoted)));
    (!found).then_some(Tool::SigningIdentity)
}

/// Where the updater key stands for signing (BLD-028, D-19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyReadiness {
    Missing,
    Locked,
    Unlocked,
}

impl KeyReadiness {
    fn missing_tool(self) -> Option<Tool> {
        match self {
            Self::Missing => Some(Tool::UpdaterKey),
            Self::Locked => Some(Tool::UpdaterKeyLocked),
            Self::Unlocked => None,
        }
    }
}

/// Whether a build process is kept from reading this process's memory, where the unlocked key is
/// (D-19): this process must not be dumpable (`PR_GET_DUMPABLE` is 0, so no core dump of it is
/// written for a step to read) and Yama's `ptrace_scope` (the text of
/// `/proc/sys/kernel/yama/ptrace_scope`, `None` when the file is missing) must be 1 or above.
pub fn memory_guarded(ptrace_scope: Option<&str>, dumpable: bool) -> bool {
    !dumpable
        && ptrace_scope
            .and_then(|text| text.trim().parse::<u8>().ok())
            .is_some_and(|scope| scope >= 1)
}

/// The PostgreSQL major version the gates start (DB-001, R-009).
const POSTGRES_MAJOR: u8 = 17;

/// The folders the gates search for `initdb` and `pg_ctl` before `PATH`
/// (`quality/src/gates/postgres.ts`).
fn postgres_dirs(path_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let major = POSTGRES_MAJOR;
    let mut dirs = vec![
        PathBuf::from(format!("/opt/homebrew/opt/postgresql@{major}/bin")),
        PathBuf::from(format!("/usr/local/opt/postgresql@{major}/bin")),
        PathBuf::from(format!("/usr/lib/postgresql/{major}/bin")),
    ];
    dirs.extend_from_slice(path_dirs);
    dirs
}

/// The pkg-config folders of the Linux distributions Anima builds on.
const PKG_CONFIG_DIRS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/pkgconfig",
    "/usr/lib64/pkgconfig",
    "/usr/lib/pkgconfig",
    "/usr/share/pkgconfig",
    "/usr/local/lib/pkgconfig",
];

/// The development packages the Tauri shell links on Linux (quality/README.md).
const LINUX_PACKAGES: &[&str] = &["webkit2gtk-4.1", "libsoup-3.0", "librsvg-2.0", "dbus-1"];

/// The Xcode command line tools, or Xcode itself, on macOS.
const MAC_DEVELOPER_DIRS: &[&str] = &[
    "/Library/Developer/CommandLineTools/usr/bin",
    "/Applications/Xcode.app/Contents/Developer/usr/bin",
];

fn tauri_packages(target: Target, env: &Environment, exists: &impl Fn(&Path) -> bool) -> bool {
    match target {
        Target::LinuxX86_64 => {
            env.find("pkg-config", exists).is_some()
                && env.find("cc", exists).is_some()
                && LINUX_PACKAGES.iter().all(|package| {
                    PKG_CONFIG_DIRS
                        .iter()
                        .any(|dir| exists(&Path::new(dir).join(format!("{package}.pc"))))
                })
        }
        Target::DarwinUniversal => MAC_DEVELOPER_DIRS
            .iter()
            .any(|dir| exists(&Path::new(dir).join("clang"))),
    }
}

/// The tools missing for `target`, in the order the Settings row lists them; `key` is where the
/// updater key stands.
pub fn missing(
    target: Target,
    env: &Environment,
    key: KeyReadiness,
    exists: impl Fn(&Path) -> bool,
) -> Vec<Tool> {
    let has = |program: &str| env.find(program, &exists).is_some();
    let postgres = postgres_dirs(env.dirs())
        .iter()
        .any(|dir| exists(&dir.join("initdb")) && exists(&dir.join("pg_ctl")));
    [
        (Tool::Git, has("git")),
        (Tool::Bun, has("bun")),
        (Tool::Rust, has("cargo") && has("rustc")),
        (Tool::Postgres, postgres),
        (Tool::TauriPackages, tauri_packages(target, env, &exists)),
    ]
    .into_iter()
    .filter_map(|(tool, present)| (!present).then_some(tool))
    .chain(key.missing_tool())
    .collect()
}
