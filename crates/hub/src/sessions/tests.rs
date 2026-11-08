//! Unit tests of the pure session rules and of the store's crate-private writes. Public
//! behaviour is tested in `tests/it/sessions`.

mod connection_input;
mod failures;
pub(super) mod fixture;
mod page;
mod probe;
mod ssh;
mod start;
mod store;
pub(super) mod tickets;
mod tunnels;

/// The key constants and the fuzz generator live with the public-key tests, in `trust`.
pub(super) use crate::trust::tests::{ED25519, ED25519_FINGERPRINT, Inputs};
