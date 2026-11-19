//! A key of the account: the base address of the operator's server and the key that the
//! server gave (K-E1 Section 7b, R-210). The base rule and the sealed layout are pure; `held`
//! opens the stored key for one request and keeps it in no cache.

use reqwest::Url;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    engine::sources::{
        Scheduling,
        model::{Account, ErrorKind},
        run::Failure,
    },
    fleet::credentials::{self, Secret, load_source_secret},
};

const VERSION: u8 = 1;
/// The longest key, in characters.
const MAX_KEY: usize = 512;
/// The longest base address that the operator gives, in characters.
const MAX_BASE: usize = 300;

/// Why a base address or a key is refused. Fixed words: never the address or the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("the address is not https://host[:port]")]
    Base,
    #[error("the key is not 1 to 512 characters with no control character")]
    Key,
}

/// The origin of the base that `text` names: `https://host[:port]`, the host in lower case and
/// no default port. Refused with a user, a password, a path other than `/`, a query or a
/// fragment. `plain_http` permits `http` too: only a transport of the tests sets it.
///
/// # Errors
/// [`Refusal::Base`].
pub fn base(text: &str, plain_http: bool) -> Result<Url, Refusal> {
    let text = text.trim();
    let url = Url::parse(text)
        .ok()
        .filter(|_| text.chars().count() <= MAX_BASE)
        .ok_or(Refusal::Base)?;
    let scheme = url.scheme() == "https" || (plain_http && url.scheme() == "http");
    let bare = url.username().is_empty()
        && url.password().is_none()
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none();
    if !scheme || !bare || url.host_str().is_none() {
        return Err(Refusal::Base);
    }
    Url::parse(&url.origin().ascii_serialization()).map_err(|_| Refusal::Base)
}

/// The text of the base as `sources.accounts.instance_key` holds it: its origin.
#[must_use]
pub fn instance_key(base: &Url) -> String {
    base.origin().ascii_serialization()
}

/// The key as the operator gave it, when it can be a key.
///
/// # Errors
/// [`Refusal::Key`].
pub fn key(text: &str) -> Result<&str, Refusal> {
    let text = text.trim();
    let valid = !text.is_empty()
        && text.chars().count() <= MAX_KEY
        && !text.chars().any(|c| c.is_control() || c.is_whitespace());
    if valid { Ok(text) } else { Err(Refusal::Key) }
}

#[derive(Serialize)]
struct Stored<'a> {
    v: u8,
    base: &'a str,
    api_key: &'a str,
}

#[derive(Deserialize)]
struct Opened {
    v: u8,
    base: String,
    api_key: String,
}

/// The plaintext to seal: `{"v":1,"base","api_key"}`, in a buffer that is wiped on drop.
///
/// # Panics
/// Never: strings and a number always serialise.
#[must_use]
pub fn to_secret(base: &Url, api_key: &str) -> Secret {
    let origin = instance_key(base);
    let stored = Stored {
        v: VERSION,
        base: &origin,
        api_key,
    };
    let worst = 6 * (origin.len() + api_key.len()) + 64;
    let mut plain = Zeroizing::new(Vec::with_capacity(worst));
    serde_json::to_writer(&mut *plain, &stored)
        .expect("strings and a number always serialise (CQ-023)");
    Secret::new(std::mem::take(&mut *plain))
}

/// The base and the key of a stored secret, checked against the account: the base must be the
/// `instance_key`, so a database writer cannot point the key at a different server. `None`
/// for an unknown layout or a base of a different account.
#[must_use]
pub fn from_secret(secret: &Secret, instance_key: &str, plain_http: bool) -> Option<(Url, Secret)> {
    let opened: Opened = serde_json::from_slice(secret.expose()).ok()?;
    let api_key = Zeroizing::new(opened.api_key);
    if opened.v != VERSION || opened.base != instance_key {
        return None;
    }
    let url = base(&opened.base, plain_http).ok()?;
    Some((url, Secret::new(api_key.as_bytes().to_vec())))
}

/// The base and the key of `account`, opened for one request.
pub(in crate::engine::sources) async fn held(
    scheduling: &Scheduling,
    account: &Account,
) -> Result<(Url, Secret), Failure> {
    let reconnect = |words| Failure::new(ErrorKind::Auth, words);
    let credential = account
        .credential_id
        .ok_or(reconnect("No key is stored. Connect the account again."))?;
    let stored = (credential, account.id);
    let secret = load_source_secret(&scheduling.pool, &scheduling.vault, stored)
        .await
        .map_err(|error| match error {
            credentials::Error::Database(_) => Failure::store(),
            _ => reconnect("The stored key cannot be opened. Connect the account again."),
        })?;
    let plain = scheduling.transport.sends_plain_http();
    from_secret(&secret, &account.instance_key, plain).ok_or(reconnect(
        "The stored key does not belong to this account. Connect the account again.",
    ))
}
