//! What a source provider and the engine say to each other (SRC-001, SRC-006, REPO-E-002).
//!
//! Pure types, no I/O and no hub imports. A provider crate depends on this crate only, so it
//! cannot reach the network, the database or a secret: it names the request it wants and
//! reads the reply the engine hands back.

mod declaration;
mod exchange;

pub use declaration::{
    Acquisition, AppKey, Declaration, MIN_POLL_INTERVAL_SECONDS, Minted, Minting,
};
pub use exchange::{Method, Normalised, Problem, Request, RequestError, Response};

#[cfg(test)]
mod tests;
