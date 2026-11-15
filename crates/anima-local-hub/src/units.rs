//! The unit that keeps `anima-local-hub run` up for one user: a `LaunchAgent` on macOS, a
//! systemd user unit on Linux (the pattern of the agent, MET-019).

pub mod install;
mod steps;
mod unit_file;

pub use steps::{STEP_LIMIT, Step, run_step};
pub use unit_file::{Launch, RenderError};
