//! Device web views and their certificate pins (WEB-008, D-4, D-5): the fingerprint form,
//! the verdict on a server-trust challenge and the leaf-certificate probe for the
//! system-browser mode (D-9).
//!
//! URLs, hosts and fingerprints are never logged and never part of an error (LOG-007).

mod certificate;
mod fingerprint;
mod pin;

#[cfg(test)]
mod tests;

pub use certificate::{CertificateError, leaf_fingerprint};
pub use fingerprint::{Fingerprint, FingerprintError};
pub use pin::{Challenge, OriginError, Pin, PinnedOrigin, Verdict, verdict};
