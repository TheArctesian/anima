//! The files tunnel (RA-032, K4 Section 9): a WebSocket at `/files` that lists, downloads and
//! uploads files on the device of a live SSH session, over the hub's own SSH link with only
//! the `sftp` subsystem (P2-003, SEC-007). It ends when that session ends.
//!
//! Pure rules: the messages (`protocol`), the transfer in flight and its byte counts
//! (`transfer`). I/O: the tickets `openFiles` issues (`tickets`, `opening`), the `/files`
//! route and its refusals (`route`), the device's file system over SFTP (`device`), the
//! running tunnel (`relay`, `requests`) and the audit entry and event of each transfer
//! (`trail`). The GraphQL part is `graphql`.
//!
//! No log line, error or event holds a host, an address, a path, the device's text, file
//! content or a token (LOG-007); the audit entry holds the path and the size, never content.

mod device;
mod graphql;
mod opening;
mod protocol;
mod relay;
mod requests;
mod route;
#[cfg(test)]
mod tests;
mod tickets;
mod trail;
mod transfer;

pub use graphql::Mutation as FilesMutation;
pub use route::files_router;
pub(super) use tickets::Tickets;
