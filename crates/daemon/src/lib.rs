//! The Anima push daemon: it reads this computer's metrics and pushes them to the hub over
//! HTTPS, or over HTTP on the Unix socket of `anima-link` (MET-011 to MET-020,
//! `contracts/daemon-push.md`).
//!
//! Pure core: [`cli`], [`config`] (origin, file contents), [`hub::answer`], [`push::pacing`],
//! [`push::renewal`], [`keys::cache`] and [`sampling`] decide and parse from their parameters. I/O sits at the
//! edge: [`hub`] (HTTP), [`config::home`], [`token_store`] and [`keys::store`] (files, Keychain), [`readers`]
//! (commands, `/proc`), [`push`] (the buffer and its rhythm) and [`runtime`] (tasks, clock,
//! signals).

pub mod cli;
pub mod config;
pub mod hub;
pub mod keys;
#[cfg(unix)]
pub mod model;
pub mod push;
pub mod readers;
pub mod report;
pub mod runtime;
pub mod sampling;
#[cfg(unix)]
pub mod threads;
pub mod token_store;
#[cfg_attr(windows, path = "units/windows.rs")]
pub mod units;

#[cfg(test)]
mod test_links;
pub mod work;
