//! A folder of stand-in programs and a `Platform` that names them.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use tempfile::TempDir;

use crate::stay_awake::platform::{Os, Platform};

pub struct Rig {
    pub dir: TempDir,
    pub platform: Platform,
}

impl Rig {
    /// A computer of `os` whose programs all succeed: the hold programs wait, `sudo` and
    /// `osascript` write their arguments to `calls`, and `pmset -g` reports `SleepDisabled 0`.
    pub fn new(os: Os) -> Self {
        let dir = tempfile::tempdir().expect("a folder");
        let at = |name: &str| dir.path().join(name).display().to_string();
        let calls = at("calls");
        let waits = "exec sleep 30";
        let records = |name: &str| format!("echo \"{name} $*\" >> '{calls}'");
        let platform = Platform {
            os,
            caffeinate: program(dir.path(), "caffeinate", waits),
            inhibit: program(dir.path(), "inhibit", waits),
            pmset: program(dir.path(), "pmset", "echo ' SleepDisabled 0'"),
            sudo: program(dir.path(), "sudo", &records("sudo")),
            osascript: program(dir.path(), "osascript", &records("osascript")),
            shell: "/bin/sh".into(),
            rule: dir.path().join("rule"),
            desktop: "GNOME".into(),
            followed: std::process::id(),
            user: 501,
        };
        Self { dir, platform }
    }

    /// A Mac on which the set-up step is done.
    pub fn mac_set_up() -> Self {
        let rig = Self::new(Os::Mac);
        fs::write(&rig.platform.rule, "rule").expect("the rule");
        rig
    }

    /// Replaces the stand-in `name` with one that runs `body`.
    pub fn stand_in(&self, name: &str, body: &str) -> String {
        program(self.dir.path(), name, body)
    }

    pub fn folder(&self) -> PathBuf {
        self.dir.path().join("config")
    }

    pub fn marker(&self) -> PathBuf {
        fs::create_dir_all(self.folder()).expect("the folder");
        crate::stay_awake::saved::marker(&self.folder())
    }

    /// What `sudo` and `osascript` were run with, one line for each run.
    pub fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The `sudo` runs only, with the stand-in `pmset` path cut to its name.
    pub fn sudo_calls(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| call.starts_with("sudo "))
            .map(|call| call.replace(&self.platform.pmset, "pmset"))
            .collect()
    }
}

fn program(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the stand-in");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("its mode");
    path.display().to_string()
}

/// A process that ends by itself after about a second, for a hold to follow.
pub fn short_lived() -> Child {
    Command::new("sleep").arg("1").spawn().expect("a process")
}

/// Waits up to five seconds for `done`.
pub fn eventually(mut done: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        if done() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    done()
}

/// Whether a process runs whose command line holds `text`.
pub fn running(text: &str) -> bool {
    Command::new("pgrep")
        .args(["-f", text])
        .output()
        .is_ok_and(|output| output.status.success())
}
