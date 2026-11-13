//! `anima`, the command line for an Anima hub (docs/cli-mcp-plan.md). One folder for each
//! operator noun (CMD-002); `hub` is the only network code (CMD-003).

pub mod api;
pub mod auth;
pub mod builders;
pub mod checks;
pub mod cli;
pub mod contexts;
pub mod devices;
pub mod environment;
pub mod events;
pub mod failure;
pub mod hub;
pub mod invocation;
pub mod keys;
pub mod maintenance;
pub mod notifications;
pub mod output;
pub mod private_file;
pub mod prompt;
pub mod releases;
pub mod retention;
pub mod vpn;
