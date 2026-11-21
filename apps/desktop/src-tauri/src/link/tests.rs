//! `tests/forms.rs` holds the pure reads and the state; `tests/runs.rs` runs a stand-in
//! `anima-link device`; `tests/swaps.rs` joins with a stand-in and swaps the link's folders.
//! `tests/kept.rs` asks a stand-in link for its unit and watches a kept link.
//! `tests/repairs.rs` puts the unit of a kept link back; `tests/ports.rs` asks a stand-in for
//! the open ports; `tests/updates.rs` updates the copy of a running stand-in link;
//! `tests/sockets.rs` checks the socket for the daemon that the app gives the link.
//! No test reaches the network: the only sockets are loopback ones the tests open.

mod forms;
mod kept;
mod ports;
mod repairs;
mod runs;
mod sockets;
mod swaps;
mod updates;
