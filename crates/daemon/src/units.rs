//! The unit files that keep the daemon running, and `anima-daemon install` (MET-019, D-10): it
//! fills the running binary's path into these texts, writes the file and starts it. The
//! operator's page command and the desktop app both call it. `anima-daemon uninstall` takes the
//! user unit out again ([`removal`], HMD-056).
//! The files are in `crates/daemon/units/`; the two systemd files end in `.unit` here only because
//! the repository's name rules ban the word `service`: `install` writes them as
//! `anima-daemon.service`.

pub mod folders;
pub mod install;
pub mod plan;
pub mod ready;
pub mod removal;
pub mod render;
mod sandbox;
pub mod session;
pub mod step;
pub mod token_file;
mod trust;
#[cfg(test)]
mod trust_cases;

/// A systemd user unit: `~/.config/systemd/user/anima-daemon.service`, run by the operator.
pub const SYSTEMD_USER: &str = include_str!("../units/anima-daemon.user.unit");

/// A systemd system unit: `/etc/systemd/system/anima-daemon.service`, run by the `anima-daemon`
/// user with its own state directory and a sandbox.
pub const SYSTEMD_SYSTEM: &str = include_str!("../units/anima-daemon.system.unit");

/// A `LaunchAgent`: `~/Library/LaunchAgents/ing.anima.daemon.plist`.
pub const LAUNCH_AGENT: &str = include_str!("../units/ing.anima.daemon.plist");
