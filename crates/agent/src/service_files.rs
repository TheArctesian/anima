//! The text of the service files that keep the agent running (MET-019). T6's installer (the
//! Tauri shell and `lib/metrics/`) copies the binary, fills in its path and writes these.
//! The files themselves are in `crates/agent/service/`.

/// A systemd user unit: `~/.config/systemd/user/anima-agent.service`, run by the operator.
pub const SYSTEMD_USER: &str = include_str!("../service/anima-agent.user.service");

/// A systemd system unit: `/etc/systemd/system/anima-agent.service`, run by the `anima-agent`
/// user with its own state directory and a sandbox.
pub const SYSTEMD_SYSTEM: &str = include_str!("../service/anima-agent.system.service");

/// A `LaunchAgent`: `~/Library/LaunchAgents/ing.anima.agent.plist`.
pub const LAUNCH_AGENT: &str = include_str!("../service/ing.anima.agent.plist");
