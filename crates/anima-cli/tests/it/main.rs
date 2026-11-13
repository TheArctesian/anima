//! `anima` run as a program against a scripted hub on a loopback port. The tests keep tokens
//! in the 0600 token file (`ANIMA_TOKEN_STORE=file`), never in this computer's key store.
//!
//! The folders mirror the command groups in `src/cli/`: `fleet`, `hub_records`, and the
//! program's own commands in `basics`. One module for each noun inside them.

mod basics;
mod coverage;
mod fleet;
mod hub_records;
mod kit;
