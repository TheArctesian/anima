//! `tests/forms.rs` holds the pure reads and the state; `tests/runs.rs` runs stand-in
//! `anima-link` scripts in temporary folders. No test reaches the network: the only sockets are
//! loopback ones the tests open.

mod forms;
mod runs;
