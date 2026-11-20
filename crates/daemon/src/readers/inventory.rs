//! Runs the probes of the tool inventory (CAP-020 to CAP-028): finds each program in the fixed
//! tool folders and `PATH`, runs it with its fixed arguments, and hands what it printed to
//! `daemon_core::inventory`, which reads it. This is the I/O edge.
//!
//! A probe gets no input, 5 seconds, 64 KiB of output, the working folder `/` and an
//! environment of only `PATH` (the folders searched), `LC_ALL=C` and `RUSTUP_AUTO_INSTALL=0`
//! (a rustup proxy must not download a toolchain). It has no home folder, except `claude` and
//! `codex`, which write into theirs even to print a version: each gets an empty temporary
//! folder as `HOME`, `CODEX_HOME` and `CLAUDE_CONFIG_DIR`, removed when it ends, so the
//! operator's `~/.claude` and `~/.codex` are never read or changed (CAP-025). On macOS without
//! the developer tools, Apple's placeholder `git` and `xcodebuild` are not run, since each
//! would offer to install the tools. Nothing a probe prints is logged.

use std::env;
#[cfg(unix)]
use std::ffi::OsStr;
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt as _;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use daemon_core::inventory::{
    Architecture, Inventory, Os, Outcome, PROBES, Program, TARGETS, TOOL_FOLDERS, candidates,
    kernel_release, macos_version, may_run, os_release_version,
};
use tempfile::TempDir;
use tokio::process::Command;

use super::Platform;
use super::command::{Bounds, CommandError, FixedCommand, capture};

#[cfg(all(test, unix))]
mod tests;

/// The bounds of one probe (CAP-022).
const PROBE_BOUNDS: Bounds = Bounds {
    limit: Duration::from_secs(5),
    keep_cut_output: false,
    output_limit: 64 * 1024,
};

/// The most of `/etc/os-release` or the kernel release read.
const FILE_LIMIT: usize = 64 * 1024;

const SW_VERS: FixedCommand = FixedCommand {
    program: "/usr/bin/sw_vers",
    args: &["-productVersion"],
    limit: Duration::from_secs(2),
    keep_cut_output: false,
};

/// Prints the folder of the macOS developer tools, or fails without them; it never offers to
/// install them.
const XCODE_SELECT: FixedCommand = FixedCommand {
    program: "/usr/bin/xcode-select",
    args: &["-p"],
    limit: Duration::from_secs(2),
    keep_cut_output: false,
};

/// Where probes look and what they may run.
struct Search {
    /// The folders searched, in order.
    folders: Vec<PathBuf>,
    /// The `PATH` a probe gets.
    path: OsString,
    os: Os,
    /// Whether the macOS developer tools are present, so Apple's placeholders may run.
    developer_tools: bool,
}

/// The tool inventory of this computer; `None` when its architecture or operating system
/// version cannot be read, so nothing is sent (the hub keeps what it has).
pub async fn inventory(platform: Platform) -> Option<Inventory> {
    let architecture = Architecture::parse(env::consts::ARCH)?;
    let os = match platform {
        Platform::Linux => Os::Linux,
        Platform::Macos => Os::Macos,
        #[cfg(windows)]
        Platform::Windows => Os::Windows,
    };
    let os_version = os_version(os).await?;
    let search = Search::here(os).await;
    Some(probe_all((architecture, os, os_version), &search).await)
}

impl Search {
    /// The search of this daemon: its `HOME` and `PATH`, and whether the developer tools are
    /// present.
    async fn here(os: Os) -> Self {
        let home =
            env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
        let folders = candidates(
            env::var_os("PATH").as_deref(),
            home.as_deref(),
            if cfg!(windows) {
                &[
                    "~/.cargo/bin",
                    "~/.bun/bin",
                    "~/.local/bin",
                    "~/AppData/Roaming/npm",
                ]
            } else {
                &TOOL_FOLDERS
            },
        );
        let path = env::join_paths(&folders).unwrap_or_default();
        let developer_tools = os == Os::Macos && developer_tools().await;
        Self {
            folders,
            path,
            os,
            developer_tools,
        }
    }
}

/// Whether the macOS developer tools are present: `xcode-select -p` prints a folder that is
/// there.
async fn developer_tools() -> bool {
    let Ok(printed) = XCODE_SELECT.run().await else {
        return false;
    };
    #[cfg(unix)]
    let folder = OsStr::from_bytes(printed.trim_ascii_end());
    #[cfg(windows)]
    let folder = std::str::from_utf8(printed.trim_ascii_end()).unwrap_or_default();
    tokio::fs::metadata(folder)
        .await
        .is_ok_and(|found| found.is_dir())
}

/// Runs every probe of `system`'s operating system and reads the inventory they give.
async fn probe_all(system: (Architecture, Os, String), search: &Search) -> Inventory {
    let mut outcomes = Vec::new();
    for probe in PROBES.iter().filter(|probe| probe.runs_on(system.1)) {
        outcomes.push((probe, run(probe.program, probe.own_home, search).await));
    }
    let targets = match run(TARGETS, false, search).await {
        Outcome::Printed(output) => Some(output),
        Outcome::Missing | Outcome::TimedOut | Outcome::Exited => None,
    };
    Inventory::new(system, &outcomes, targets.as_deref())
}

/// The operating system version: `sw_vers` on macOS; on Linux the distribution's version, or
/// the kernel release for a distribution that has none.
async fn os_version(os: Os) -> Option<String> {
    match os {
        Os::Windows => windows_version(),
        Os::Macos => macos_version(&SW_VERS.run().await.ok()?),
        Os::Linux => {
            let release = read_capped(Path::new("/etc/os-release")).await;
            if let Some(version) = release.as_deref().and_then(os_release_version) {
                return Some(version);
            }
            kernel_release(&read_capped(Path::new("/proc/sys/kernel/osrelease")).await?)
        }
    }
}

async fn read_capped(path: &Path) -> Option<Vec<u8>> {
    let mut bytes = tokio::fs::read(path).await.ok()?;
    bytes.truncate(FILE_LIMIT);
    Some(bytes)
}

/// Finds `program` and runs it, with an empty home folder of its own when `own_home`:
/// `Missing` when no folder has it, `TimedOut` when it ran past 5 seconds, `Exited` when it
/// could not start or exited with a failure.
async fn run(program: Program, own_home: bool, search: &Search) -> Outcome {
    let Some(path) = find(program.name, search).await else {
        return Outcome::Missing;
    };
    let mut command = Command::new(path);
    command
        .args(program.args)
        .env_clear()
        .env("PATH", &search.path)
        .env("LC_ALL", "C")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .current_dir(if cfg!(windows) {
            env::var_os("SystemRoot").unwrap_or_else(|| "C:/Windows".into())
        } else {
            "/".into()
        });
    #[cfg(windows)]
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    let home = match own_home.then(|| give_home(&mut command)) {
        // Without a folder of its own the probe does not run: never with the operator's home.
        Some(None) => return Outcome::Exited,
        Some(home) => home,
        None => None,
    };
    let outcome = outcome(capture(&mut command, program.name, PROBE_BOUNDS).await);
    if let Some(home) = home {
        remove(home);
    }
    outcome
}

/// Gives `command` a new empty folder as its home and the config folders of `claude` and
/// `codex` (CAP-025); `None` when no folder could be made.
fn give_home(command: &mut Command) -> Option<TempDir> {
    let home = tempfile::tempdir()
        .inspect_err(|error| tracing::debug!(%error, "no home folder for a tool probe"))
        .ok()?;
    for name in [
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
    ] {
        command.env(name, home.path());
    }
    Some(home)
}

fn remove(home: TempDir) {
    if let Err(error) = home.close() {
        tracing::debug!(%error, "a tool probe's home folder was not removed");
    }
}

fn outcome(captured: Result<Vec<u8>, CommandError>) -> Outcome {
    match captured {
        Ok(output) => Outcome::Printed(output),
        Err(error) => {
            tracing::debug!(%error, "a tool probe gave no version");
            match error {
                CommandError::Timeout { .. } => Outcome::TimedOut,
                CommandError::Start { .. } | CommandError::Failed { .. } => Outcome::Exited,
            }
        }
    }
}

/// The first folder of `search` that holds an executable file `name`, links followed, that
/// may run (see [`may_run`]).
async fn find(name: &str, search: &Search) -> Option<PathBuf> {
    for path in probe_paths(&search.folders, name) {
        if let Ok(found) = tokio::fs::metadata(&path).await
            && found.is_file()
            && executable(&found)
            && may_run(&path, search.os, search.developer_tools)
        {
            return Some(path);
        }
    }
    None
}

fn probe_paths(folders: &[PathBuf], name: &str) -> Vec<PathBuf> {
    folders
        .iter()
        .flat_map(|folder| {
            if cfg!(windows) {
                vec![
                    folder.join(format!("{name}.exe")),
                    folder.join(format!("{name}.cmd")),
                ]
            } else {
                vec![folder.join(name)]
            }
        })
        .collect()
}

fn executable(found: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        found.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        found.is_file()
    }
}

fn windows_version() -> Option<String> {
    #[cfg(windows)]
    {
        sysinfo::System::os_version()
            .ok()
            .filter(|value| daemon_core::inventory::is_version(value))
    }
    #[cfg(not(windows))]
    {
        None
    }
}
