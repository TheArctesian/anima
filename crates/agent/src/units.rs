//! The unit files that keep the agent running, and `anima-agent install` (MET-019, D-10): it
//! fills the running binary's path into these texts, writes the file and starts it. The
//! operator's page command and the desktop app both call it.
//! The files are in `crates/agent/units/`; the two systemd files end in `.unit` here only because
//! the repository's name rules ban the word `service`: `install` writes them as
//! `anima-agent.service`.

pub mod folders;
pub mod install;
pub mod plan;
pub mod ready;
pub mod render;
mod sandbox;
pub mod session;
pub mod step;
pub mod token_file;
mod trust;
#[cfg(test)]
mod trust_cases;

/// A systemd user unit: `~/.config/systemd/user/anima-agent.service`, run by the operator.
pub const SYSTEMD_USER: &str = include_str!("../units/anima-agent.user.unit");

/// A systemd system unit: `/etc/systemd/system/anima-agent.service`, run by the `anima-agent`
/// user with its own state directory and a sandbox.
pub const SYSTEMD_SYSTEM: &str = include_str!("../units/anima-agent.system.unit");

/// A `LaunchAgent`: `~/Library/LaunchAgents/ing.anima.agent.plist`.
pub const LAUNCH_AGENT: &str = include_str!("../units/ing.anima.agent.plist");
