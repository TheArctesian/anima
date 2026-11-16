//! This computer as the stay-awake holds need it: the OS, its fixed programs and the process
//! the holds follow. A test gives its own programs, so no test changes a power setting.

use std::path::PathBuf;

/// The two desktop systems, which keep a computer awake in different ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Linux,
}

/// The programs are absolute paths on macOS, so `PATH` cannot swap them. On Linux
/// `systemd-inhibit` and `tail` are in a different folder on each distribution, so `PATH`
/// finds them; they run with this user's rights only.
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
    /// Holds the idle-sleep assertion on macOS (AWK-002).
    pub caffeinate: String,
    /// Holds a `logind` inhibitor on Linux.
    pub inhibit: String,
    /// Reads and writes the macOS power settings.
    pub pmset: String,
    /// Runs `pmset` as root through the rule of the set-up step, never with a password.
    pub sudo: String,
    /// Asks macOS for an administrator, for the set-up step only.
    pub osascript: String,
    /// Runs the watcher that restores the sleep setting when the app ends (AWK-007).
    pub shell: String,
    /// The rule file of the set-up step.
    pub rule: PathBuf,
    /// `XDG_CURRENT_DESKTOP`: some desktops decide the lid themselves.
    pub desktop: String,
    /// The process the holds follow: they end when it ends.
    pub followed: u32,
    /// The user the rule names.
    pub user: u32,
}

impl Platform {
    /// This computer and this process.
    #[must_use]
    pub fn here() -> Self {
        Self {
            os: if cfg!(target_os = "macos") {
                Os::Mac
            } else {
                Os::Linux
            },
            caffeinate: "/usr/bin/caffeinate".into(),
            inhibit: "systemd-inhibit".into(),
            pmset: "/usr/bin/pmset".into(),
            sudo: "/usr/bin/sudo".into(),
            osascript: "/usr/bin/osascript".into(),
            shell: "/bin/sh".into(),
            rule: PathBuf::from("/private/etc/sudoers.d/anima-stay-awake"),
            desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
            followed: std::process::id(),
            user: nix::unistd::getuid().as_raw(),
        }
    }
}
