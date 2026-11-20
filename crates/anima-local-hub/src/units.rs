//! The unit that keeps `anima-local-hub run` up for one user: a `LaunchAgent` on macOS, a
//! systemd user unit on Linux (the pattern of the agent, MET-019).

mod change_lock;
pub mod install;
pub mod listen;
mod request;
mod steps;
mod trust;
mod unit_file;
pub mod update;

pub use steps::{STEP_LIMIT, Step, run_step};
pub use unit_file::{Launch, RenderError};

#[cfg(windows)]
mod windows;
