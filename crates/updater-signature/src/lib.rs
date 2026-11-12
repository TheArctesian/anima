//! The tauri-updater signature of an Anima release (BLD-045, BLD-046, UPD-002, R-116): open the
//! minisign secret key file `tauri signer generate` writes, sign the BLAKE2b-512 hash of an
//! updater file with the version in the trusted comment, and check a `.sig` text with the
//! verifier tauri-plugin-updater itself uses. In-house minisign (WP-36 D-11): ed25519-dalek,
//! blake2 and scrypt; no `minisign` crate outside the tests.
//!
//! Pure: no file, no clock, no log. The hub reads the key file at start and passes the bytes in.
//! The secret key has no `Debug`, `Display` or `Clone`, and its seed is wiped when it drops; an
//! error is a variant with fixed text (LOG-007).

mod error;
mod key_file;
mod sealed;
mod signature;

#[cfg(test)]
mod tests;

pub use error::SignatureError;
pub use key_file::{PublicKey, SecretKey};
pub use signature::{Prehash, SignatureCheck, SignatureStream, Signed, verify};
