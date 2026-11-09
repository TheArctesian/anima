//! The agent against a real local HTTP server standing in for the hub (K8). The origin is a
//! parameter, so the tests use plain `http` on 127.0.0.1; the https-only rule is proved by the
//! origin parser's own tests. The tests that keep a token use the Linux token file.

#[cfg(target_os = "linux")]
mod enrol;
mod fake_hub;
#[cfg(target_os = "linux")]
mod kit;
#[cfg(target_os = "linux")]
mod push;
#[cfg(target_os = "linux")]
mod readers;
#[cfg(target_os = "linux")]
mod renewal;
#[cfg(target_os = "linux")]
mod token_store;
mod transport;
mod units;
