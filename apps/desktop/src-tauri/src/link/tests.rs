//! `tests/forms.rs` holds the pure reads and the state; `tests/runs.rs` runs a stand-in
//! `anima-link device`; `tests/swaps.rs` joins with a stand-in and swaps the link's folders.
//! `tests/kept.rs` asks a stand-in link for its unit and watches a kept link.
//! `tests/repairs.rs` puts the unit of a kept link back; `tests/ports.rs` asks a stand-in for
//! the open ports.
//! No test reaches the network: the only sockets are loopback ones the tests open.

mod forms;
mod kept;
mod ports;
mod repairs;
mod runs;
mod swaps;
