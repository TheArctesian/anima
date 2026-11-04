use keyring::Entry;

use super::error::KeystoreError;

pub(super) const RELEASE_SERVICE: &str = "ing.anima.desktop";

/// Release builds use the shipped service name; debug builds use their own so a development
/// build never reads or overwrites a released app's token.
pub(super) const SERVICE: &str = if cfg!(debug_assertions) {
    "ing.anima.desktop.dev"
} else {
    RELEASE_SERVICE
};

const SCHEME: &str = "https://";

/// Input check only, no URL resolution (REPO-003): `https://` then a lowercase host (letters,
/// digits, `.`, `-`, `_`, IPv6 brackets) with an optional port, as `parseHubAddress` produces.
/// Anything with a path, query, whitespace, uppercase or trailing slash is not the canonical
/// origin the client sends.
fn is_canonical_origin(origin: &str) -> bool {
    origin.strip_prefix(SCHEME).is_some_and(|host| {
        !host.is_empty()
            && host
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || ".-_:[]".contains(c))
    })
}

/// The entry for one hub. The account is the hub origin, so a token is only ever read back for
/// the hub that issued it.
fn entry_for(origin: &str) -> Result<Entry, KeystoreError> {
    if !is_canonical_origin(origin) {
        return Err(KeystoreError::BadOrigin);
    }
    Entry::new(SERVICE, origin).map_err(KeystoreError::from)
}

/// Reads the token stored for `origin`. A missing entry is `None`.
pub fn read_token(origin: &str) -> Result<Option<String>, KeystoreError> {
    match entry_for(origin)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Stores the token for `origin`, replacing any earlier one. An empty token is refused.
pub fn write_token(origin: &str, token: &str) -> Result<(), KeystoreError> {
    let entry = entry_for(origin)?;
    if token.is_empty() {
        return Err(KeystoreError::EmptyToken);
    }
    entry.set_password(token).map_err(KeystoreError::from)
}

/// Removes the token stored for `origin`. A missing entry is already cleared.
pub fn clear_token(origin: &str) -> Result<(), KeystoreError> {
    match entry_for(origin)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
