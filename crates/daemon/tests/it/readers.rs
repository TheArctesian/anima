//! The readers of the daemon: the machine that runs the tests, and the container of a hosted hub.

mod container;
#[cfg(target_os = "linux")]
mod machine;
