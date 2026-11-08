//! The environment every build step runs in (BLD-044). Build scripts, `build.rs`, proc-macros
//! and npm packages run in these steps, so the step gets an allow-list of the app's variables and
//! nothing else: no updater key, no builder or hub token, no URL with a secret, no
//! `DATABASE_URL` (BLD-042). Pure: the app's environment is a parameter.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Variables passed through from the app. Everything else is dropped.
const PASSED: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TMPDIR",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
];

/// Set for every step: CI behaviour, no colour codes, no prompts, and git with no user or system
/// configuration (a credential helper or `url.insteadOf` there could redirect the clone).
const FIXED: &[(&str, &str)] = &[
    ("CI", "true"),
    ("NO_COLOR", "1"),
    ("CARGO_TERM_COLOR", "never"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
];

/// Folders searched after the app's `PATH`: an app started from the Dock or a desktop menu often
/// lacks the ones rustup, bun and Homebrew install into.
const EXTRA_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"];

/// The variables and program folders of a build step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    passed: Vec<(OsString, OsString)>,
    dirs: Vec<PathBuf>,
}

impl Environment {
    /// The allow-listed part of `parent`, the app's environment.
    pub fn from_parent(parent: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        let parent: Vec<(OsString, OsString)> = parent.into_iter().collect();
        let value = |name: &str| {
            parent
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        };
        let passed = PASSED
            .iter()
            .filter_map(|name| value(name).map(|value| (OsString::from(name), value)))
            .collect();
        let dirs = search_dirs(
            value("PATH").as_deref(),
            value("HOME").as_deref(),
            value("CARGO_HOME").as_deref(),
        );
        Self { passed, dirs }
    }

    /// Program folders, in search order. Each is absolute.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// The full environment of one step: the passed variables, `PATH`, the fixed ones, then the
    /// step's own (`BASE_REF`, for example).
    pub fn for_step(&self, step: &[(&'static str, String)]) -> Vec<(OsString, OsString)> {
        let mut vars = self.passed.clone();
        let path = std::env::join_paths(&self.dirs).unwrap_or_default();
        vars.push(("PATH".into(), path));
        vars.extend(
            FIXED
                .iter()
                .map(|(k, v)| (OsString::from(k), OsString::from(v))),
        );
        vars.extend(
            step.iter()
                .map(|(k, v)| (OsString::from(k), OsString::from(v))),
        );
        vars
    }

    /// The first `program` in the program folders for which `exists` holds.
    pub fn find(&self, program: &str, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
        self.dirs
            .iter()
            .map(|dir| dir.join(program))
            .find(|path| exists(path))
    }
}

/// The app's `PATH`, then cargo's and bun's folders and `EXTRA_DIRS`, without repeats. A relative
/// entry (`.`, an empty one) is dropped: it would find programs in the clone, which the build
/// writes.
fn search_dirs(path: Option<&OsStr>, home: Option<&OsStr>, cargo: Option<&OsStr>) -> Vec<PathBuf> {
    let home = home.map(PathBuf::from).filter(|home| home.is_absolute());
    let cargo_bin = cargo
        .map(|cargo| PathBuf::from(cargo).join("bin"))
        .or_else(|| home.as_ref().map(|home| home.join(".cargo/bin")));
    let bun_bin = home.as_ref().map(|home| home.join(".bun/bin"));
    let mut dirs: Vec<PathBuf> = Vec::new();
    let candidates = path
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .chain(cargo_bin)
        .chain(bun_bin)
        .chain(EXTRA_DIRS.iter().map(PathBuf::from));
    for dir in candidates {
        if dir.is_absolute() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}
