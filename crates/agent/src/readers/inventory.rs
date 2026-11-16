//! Runs the probes of the tool inventory (CAP-020 to CAP-028): finds each program in `PATH` and
//! the fixed tool folders, runs it with its fixed arguments, and hands what it printed to
//! `agent_core::inventory`, which reads it. This is the I/O edge.
//!
//! A probe gets no input, 5 seconds, 64 KiB of output, the working folder `/` and an
//! environment of only `PATH` (the folders searched), `LC_ALL=C` and `RUSTUP_AUTO_INSTALL=0`
//! (a rustup proxy must not download a toolchain). No file of a tool is read (CAP-025), and
//! nothing a probe prints is logged.

use std::env;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_core::inventory::{
    Architecture, Inventory, Os, Outcome, PROBES, Program, TARGETS, TOOL_FOLDERS, candidates,
    kernel_release, macos_version, os_release_version,
};
use tokio::process::Command;

use super::Platform;
use super::command::{Bounds, FixedCommand, capture};

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

/// The folders searched and the `PATH` a probe gets.
struct Search {
    folders: Vec<PathBuf>,
    path: OsString,
}

/// The tool inventory of this computer; `None` when its architecture or operating system
/// version cannot be read, so nothing is sent (the hub keeps what it has).
pub async fn inventory(platform: Platform) -> Option<Inventory> {
    let architecture = Architecture::parse(env::consts::ARCH)?;
    let os = match platform {
        Platform::Linux => Os::Linux,
        Platform::Macos => Os::Macos,
    };
    let os_version = os_version(os).await?;
    let home = env::var_os("HOME").map(PathBuf::from);
    let folders = candidates(
        env::var_os("PATH").as_deref(),
        home.as_deref(),
        &TOOL_FOLDERS,
    );
    let path = env::join_paths(&folders).unwrap_or_default();
    let search = Search { folders, path };
    let mut outcomes = Vec::new();
    for probe in PROBES.iter().filter(|probe| probe.runs_on(os)) {
        outcomes.push((probe, run(probe.program, &search).await));
    }
    let targets = match run(TARGETS, &search).await {
        Outcome::Printed(output) => Some(output),
        Outcome::Missing | Outcome::Failed => None,
    };
    let system = (architecture, os, os_version);
    Some(Inventory::new(system, &outcomes, targets.as_deref()))
}

/// The operating system version: `sw_vers` on macOS; on Linux the distribution's version, or
/// the kernel release for a distribution that has none.
async fn os_version(os: Os) -> Option<String> {
    match os {
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

/// Finds `program` and runs it: `Missing` when no folder has it, `Failed` when it could not
/// start, ran past 5 seconds or exited with a failure.
async fn run(program: Program, search: &Search) -> Outcome {
    let Some(path) = find(program.name, &search.folders).await else {
        return Outcome::Missing;
    };
    let mut command = Command::new(path);
    command
        .args(program.args)
        .env_clear()
        .env("PATH", &search.path)
        .env("LC_ALL", "C")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .current_dir("/");
    match capture(&mut command, program.name, PROBE_BOUNDS).await {
        Ok(output) => Outcome::Printed(output),
        Err(error) => {
            tracing::debug!(%error, "a tool probe gave no version");
            Outcome::Failed
        }
    }
}

/// The first `folders` entry that holds an executable file `name`, links followed.
async fn find(name: &str, folders: &[PathBuf]) -> Option<PathBuf> {
    for folder in folders {
        let path = folder.join(name);
        if let Ok(found) = tokio::fs::metadata(&path).await
            && found.is_file()
            && found.permissions().mode() & 0o111 != 0
        {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use agent_core::inventory::is_version;

    use super::*;

    /// A folder with an executable `sh` script `name` that runs `body`.
    fn script(folder: &Path, name: &str, body: &str) -> PathBuf {
        let path = folder.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
        path
    }

    fn search(folders: &[&Path]) -> Search {
        let mut folders: Vec<PathBuf> = folders.iter().map(|folder| folder.to_path_buf()).collect();
        folders.push(PathBuf::from("/bin"));
        folders.push(PathBuf::from("/usr/bin"));
        let path = env::join_paths(&folders).expect("joined");
        Search { folders, path }
    }

    const fn program(name: &'static str) -> Program {
        Program {
            name,
            args: &["--version"],
        }
    }

    #[tokio::test]
    async fn cap_021_a_program_is_the_first_executable_file_of_that_name() {
        let first = tempfile::tempdir().expect("a folder");
        let second = tempfile::tempdir().expect("a folder");
        fs::write(first.path().join("git"), "not executable").expect("written");
        fs::create_dir(first.path().join("bun")).expect("a folder named bun");
        let git = script(second.path(), "git", "echo git version 2.43.0");
        let folders = search(&[first.path(), second.path()]).folders;
        assert_eq!(find("git", &folders).await, Some(git));
        assert_eq!(find("bun", &folders).await, None);
        assert_eq!(
            run(program("anima-no-such-tool"), &search(&[])).await,
            Outcome::Missing
        );
    }

    #[tokio::test]
    async fn cap_022_a_probe_gets_no_home_no_input_and_only_the_search_path() {
        let folder = tempfile::tempdir().expect("a folder");
        script(
            folder.path(),
            "probe",
            "echo \"home=$HOME pwd=$(pwd) args=$*\"; if read line; then echo input; fi",
        );
        let Outcome::Printed(output) = run(program("probe"), &search(&[folder.path()])).await
        else {
            panic!("it ran");
        };
        assert_eq!(
            String::from_utf8_lossy(&output),
            "home= pwd=/ args=--version\n"
        );
    }

    #[tokio::test]
    async fn cap_024_a_probe_that_fails_or_runs_past_5_seconds_is_failed() {
        let folder = tempfile::tempdir().expect("a folder");
        script(folder.path(), "broken", "echo 1.0; exit 3");
        script(folder.path(), "stuck", "sleep 30");
        let found = search(&[folder.path()]);
        assert_eq!(run(program("broken"), &found).await, Outcome::Failed);
        let started = std::time::Instant::now();
        assert_eq!(run(program("stuck"), &found).await, Outcome::Failed);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn cap_026_this_computer_s_inventory_names_its_architecture_and_clean_versions() {
        let inventory = inventory(Platform::current()).await.expect("readable");
        assert_eq!(inventory.architecture.as_str(), env::consts::ARCH);
        assert!(
            is_version(&inventory.os_version),
            "{}",
            inventory.os_version
        );
        for tool in &inventory.tools {
            assert!(tool.version.as_deref().is_none_or(is_version), "{tool:?}");
        }
    }
}
