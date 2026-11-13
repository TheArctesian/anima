//! `anima`, the command line for an Anima hub (docs/cli-mcp-plan.md). One folder for each
//! operator noun (CMD-002); `hub` is the only network code (CMD-003).

pub mod api;
pub mod auth;
pub mod checks;
pub mod cli;
pub mod contexts;
pub mod devices;
pub mod environment;
pub mod failure;
pub mod hub;
pub mod invocation;
pub mod maintenance;
pub mod output;
pub mod private_file;
pub mod prompt;
