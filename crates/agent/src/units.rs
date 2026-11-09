//! The text of the unit files that keep the agent running (MET-019). T6's installer (the
//! Tauri shell and `lib/metrics/`) copies the binary, fills in its path and writes these.
//! The files are in `crates/agent/units/`; the two systemd files end in `.unit` here only because
//! the repository's name rules ban the word `service`: install them as `anima-agent.service`.

/// A systemd user unit: `~/.config/systemd/user/anima-agent.service`, run by the operator.
pub const SYSTEMD_USER: &str = include_str!("../units/anima-agent.user.unit");

/// A systemd system unit: `/etc/systemd/system/anima-agent.service`, run by the `anima-agent`
/// user with its own state directory and a sandbox.
pub const SYSTEMD_SYSTEM: &str = include_str!("../units/anima-agent.system.unit");

/// A `LaunchAgent`: `~/Library/LaunchAgents/ing.anima.agent.plist`.
pub const LAUNCH_AGENT: &str = include_str!("../units/ing.anima.agent.plist");
