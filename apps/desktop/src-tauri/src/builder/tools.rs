//! The tools a builder needs before it claims a job (BLD-028): git, bun, Rust with
//! cargo-nextest (the gates run the Rust tests with it, R-112), PostgreSQL 17 (R-009), the Tauri
//! system packages and the app's own updater public key. No updater key: the hub signs (R-116).
//! Pure: whether a path exists is a parameter, and nothing is run.

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
    /// `cargo-nextest`, which gate 3 runs the Rust tests with (R-112).
    CargoNextest,
    Postgres,
    TauriPackages,
    /// This app was built without the updater public key (`plugins.updater.pubkey` is the
    /// placeholder), so what it builds could never update (R-108).
    UpdaterPublicKey,
}

/// The length of a minisign public key: `Ed`, the 8-byte key ID and the 32-byte key.
const PUBLIC_KEY_BYTES: usize = 42;

/// The last line of the public key file whose base64 is `encoded` (`plugins.updater.pubkey`),
/// if it is text.
pub fn key_line(encoded: &str) -> Option<String> {
    let text = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_owned)
}

/// [`Tool::UpdaterPublicKey`] unless `line`, the key line of the app's updater public key, is a
/// minisign public key (R-108). The placeholder has no key line.
pub fn app_public_key(line: Option<&str>) -> Option<Tool> {
    let key = line
        .and_then(|line| STANDARD.decode(line).ok())
        .is_some_and(|bytes| bytes.len() == PUBLIC_KEY_BYTES && bytes.starts_with(b"Ed"));
    (!key).then_some(Tool::UpdaterPublicKey)
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

/// The tools missing for `target`, in the order the Settings row lists them. `psql` is not one:
/// the gates start their own PostgreSQL with `initdb` and `pg_ctl` and never run it.
pub fn missing(target: Target, env: &Environment, exists: impl Fn(&Path) -> bool) -> Vec<Tool> {
    let has = |program: &str| env.find(program, &exists).is_some();
    let postgres = postgres_dirs(env.dirs())
        .iter()
        .any(|dir| exists(&dir.join("initdb")) && exists(&dir.join("pg_ctl")));
    [
        (Tool::Git, has("git")),
        (Tool::Bun, has("bun")),
        (Tool::Rust, has("cargo") && has("rustc")),
        (Tool::CargoNextest, has("cargo-nextest")),
        (Tool::Postgres, postgres),
        (Tool::TauriPackages, tauri_packages(target, env, &exists)),
    ]
    .into_iter()
    .filter_map(|(tool, present)| (!present).then_some(tool))
    .collect()
}
