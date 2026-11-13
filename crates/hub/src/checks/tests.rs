//! Unit tests of checks: the pure rules here, configuration and probes under `probe`.
//! Behaviour against the database is tested in `tests/it/checks`.

mod beat;
mod evaluation;
mod heartbeat_cost;
mod probe;
mod push;
mod sealing;
pub(in crate::checks) mod snmp;
mod standing;
mod stored_config;
mod tailscale;
mod uptime;
