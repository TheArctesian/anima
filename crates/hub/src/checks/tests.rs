//! Tests for check configuration, probes and the pure check rules. Servers listen on
//! 127.0.0.1 only; nothing reaches the internet. Behaviour against the database is tested in
//! `tests/it/checks`.

mod config;
mod dns;
mod evaluation;
mod http;
mod icmp;
mod reach;
mod secrets;
mod standing;
mod support;
mod tcp;
mod tls;
mod uptime;
