//! The tools a builder needs before it claims a job (BLD-028): git, bun, Rust, PostgreSQL 17
//! (R-009), the Tauri system packages and the updater key. Pure: whether a path exists is a
//! parameter, and nothing is run.

use std::path::{Path, PathBuf};

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

/// The tools missing for `target`, in the order the Settings row lists them. `key_present`
/// says whether the updater key is in the key store.
pub fn missing(
    target: Target,
    env: &Environment,
    key_present: bool,
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
        (Tool::UpdaterKey, key_present),
    ]
    .into_iter()
    .filter_map(|(tool, present)| (!present).then_some(tool))
    .collect()
}
