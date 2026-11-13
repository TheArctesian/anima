//! Unit tests of the SSH client: the observer, the probe's edges, and (`link/tests`) the link
//! against a test SSH server on loopback. The fixture, the bench and the server also serve the
//! tests of `sessions` that use the link or the same world (Wake-on-LAN, the tunnels).

pub(crate) mod fixture;
mod observer;
mod probe;

pub(crate) use super::link::tests::{
    Bench, SEED, bench, bench_accepting, reaching,
    server::{Answer, Heard, Server, host_key, serve},
};
