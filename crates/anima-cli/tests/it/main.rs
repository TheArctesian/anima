//! `anima` run as a program against a scripted hub on a loopback port. The tests keep tokens
//! in the 0600 token file (`ANIMA_TOKEN_STORE=file`), never in this computer's key store.
//!
//! The folders mirror the command groups in `src/cli/`: `fleet`, `hub_records`, and the
//! program's own commands in `basics`. One module for each noun inside them. `end_to_end`
//! runs the same program against the real hub instead of the scripted one.

mod basics;
mod coverage;
mod end_to_end;
mod fleet;
mod hub_records;
mod kit;
mod mcp;
