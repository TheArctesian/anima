//! The sealed form of channel secrets (SEC-003, D-17, K3): one JSON document per channel,
//! sealed with the hub key under the context `notifications.channels:<channel_id>`, so a value
//! copied to another channel does not open.
//!
//! The plaintext also binds the secrets to where they go: the channel kind, the target (the
//! webhook origin, the ntfy server, the mail server's host and port, or the Apprise server) and
//! the webhook header names. Opening checks the binding against the stored settings, so a
//! database writer who points a channel at another server gets an error, not the secrets, and
//! nothing is sent (as checks does, D-5).
//!
//! Plaintext layout: `{"kind": .., "target": .., "names": [..], "secrets": Wire}`.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{
    Error,
    channels::{Kept, Secrets, Settings, Wire},
};
use crate::{
    fleet::credentials::{self, Sealed, Secret, Vault},
    ids::ChannelId,
};

/// The context prefix of sealed channel secrets (K3).
const CONTEXT: &str = "notifications.channels";

/// The associated data that binds sealed secrets to their channel.
fn context(channel: ChannelId) -> Vec<u8> {
    format!("{CONTEXT}:{channel}").into_bytes()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    #[serde(borrow)]
    kind: Cow<'a, str>,
    #[serde(borrow)]
    target: Cow<'a, str>,
    #[serde(borrow)]
    names: Vec<Cow<'a, str>>,
    #[serde(borrow)]
    secrets: Wire<'a>,
}

/// Seals the channel's secrets, bound to its kind, target and header names.
///
/// # Errors
/// `Credentials` (`Random`) when the OS random source fails.
pub(super) fn seal(
    vault: &Vault,
    channel: ChannelId,
    settings: &Settings,
    secrets: &Secrets,
) -> Result<Vec<u8>, Error> {
    let target = settings.target();
    let names = settings.secret_names();
    let envelope = Envelope {
        kind: Cow::Borrowed(settings.kind().as_str()),
        target: Cow::Borrowed(&target),
        names: names
            .iter()
            .map(|name| Cow::Borrowed(name.as_str()))
            .collect(),
        secrets: secrets.to_wire(),
    };
    // Sized for the worst case (every byte escaped as `\uXXXX`), so the buffer never grows
    // and leaves no copy of a secret behind.
    let clear: usize = [target.len(), names.iter().map(String::len).sum()]
        .iter()
        .sum();
    let worst = envelope.secrets.worst_len() + 6 * clear + 8 * names.len() + 128;
    let mut plain = Zeroizing::new(Vec::with_capacity(worst));
    serde_json::to_writer(&mut *plain, &envelope)
        .expect("strings and lists of strings always serialise (CQ-023)");
    let plain = Secret::new(std::mem::take(&mut *plain));
    Ok(vault.seal(&plain, &context(channel))?.to_bytes())
}

/// Opens the channel's secrets, if they were sealed for this channel and for where `settings`
/// sends them.
///
/// # Errors
/// `Credentials` when the value does not open or is malformed; `SecretsUnbound` when the
/// settings name another kind, target or header names than the secrets were given for, or
/// the secrets do not have the shape the settings need.
pub(super) fn open(
    vault: &Vault,
    channel: ChannelId,
    settings: &Settings,
    bytes: &[u8],
) -> Result<Secrets, Error> {
    let plain = vault.open(&Sealed::from_bytes(bytes)?, &context(channel))?;
    let envelope: Envelope<'_> = serde_json::from_slice(plain.expose())
        .map_err(|_| Error::Credentials(credentials::Error::Malformed))?;
    let names = settings.secret_names();
    let bound = envelope.kind == settings.kind().as_str()
        && envelope.target == settings.target()
        && envelope
            .names
            .iter()
            .map(Cow::as_ref)
            .eq(names.iter().map(String::as_str));
    if !bound {
        return Err(Error::SecretsUnbound);
    }
    Secrets::from_wire(settings, envelope.secrets).ok_or(Error::SecretsUnbound)
}

/// The stored secrets as an update finds them: nothing, opened, or present and unreadable.
pub(super) fn kept(
    vault: &Vault,
    channel: ChannelId,
    settings: &Settings,
    bytes: Option<&[u8]>,
) -> Kept<Secrets> {
    match bytes.map(|bytes| open(vault, channel, settings, bytes)) {
        None => Kept::Nothing,
        Some(Ok(secrets)) => Kept::Opened(secrets),
        Some(Err(_)) => Kept::Unopened,
    }
}
