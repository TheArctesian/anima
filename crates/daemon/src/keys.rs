//! The operational keys of this machine: a cache of the versions its hub gives it, kept where
//! the token is kept, and the local reader `anima-daemon key read` (K-E5 Section 2.1, R-147).
//!
//! Pure core: [`cache`] decides what to drop and what to fetch from the hub's answer and the
//! time, and [`record`] is the stored form. I/O at the edge: [`store`] (Keychain or a 0600
//! file) and [`keeper`] (the store and the hub). A cache is a copy: it holds one version of each
//! key, follows the hub at each push, and is deleted after 7 days with no answer that named the
//! keys, on a refused token, and when the daemon leaves the machine. No type that holds a value
//! implements `Debug` or `Display`, and no log line holds a value or a key name (LOG-007).

pub mod cache;
pub mod keeper;
pub mod record;
pub mod store;

use chrono::{DateTime, Utc};

use cache::KeyCache;
use store::{KeyStore, KeyStoreError};

/// Why `anima-daemon key read` gave no value. No message holds a value.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("standard output is a terminal; a key is written only to a pipe or a file")]
    Terminal,
    #[error("this machine holds no key of that name")]
    NotHeld,
    #[error("{0}")]
    Store(#[from] KeyStoreError),
    #[error("the key could not be written: {0}")]
    Write(std::io::ErrorKind),
}

/// Writes the cached value of the key named `name` to `out`, which must not be a terminal
/// (`to_terminal`), at `now`. A cache past its 7 days is deleted first, and gives nothing.
///
/// # Errors
/// [`ReadError`] for a terminal, a key the cache does not hold, or a store or write failure.
pub fn read(
    store: &KeyStore,
    (name, now): (&str, DateTime<Utc>),
    (out, to_terminal): (&mut dyn std::io::Write, bool),
) -> Result<(), ReadError> {
    if to_terminal {
        return Err(ReadError::Terminal);
    }
    let cache: KeyCache = store.load()?;
    if cache.is_stale(now) {
        store.forget()?;
        return Err(ReadError::NotHeld);
    }
    let value = cache.value(name).ok_or(ReadError::NotHeld)?;
    out.write_all(value.expose())
        .and_then(|()| out.flush())
        .map_err(|error| ReadError::Write(error.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::home::Home;

    #[test]
    fn r_147_key_read_refuses_a_terminal_before_it_opens_the_store() {
        let home = Home::at(std::path::PathBuf::from("/nonexistent/anima-daemon"));
        let mut out = Vec::new();
        let asked = ("cloudflare.dns", DateTime::UNIX_EPOCH);
        let refused = read(&KeyStore::open(&home), asked, (&mut out, true));
        assert!(matches!(refused, Err(ReadError::Terminal)), "{refused:?}");
        assert_eq!(out, Vec::<u8>::new());
    }
}
