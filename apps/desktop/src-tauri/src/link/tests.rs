//! `tests/forms.rs` holds the pure reads and the state; `tests/runs.rs` runs a stand-in
//! `anima-link device`; `tests/swaps.rs` joins with a stand-in and swaps the link's folders.
//! No test reaches the network: the only sockets are loopback ones the tests open.

mod forms;
mod runs;
mod swaps;
