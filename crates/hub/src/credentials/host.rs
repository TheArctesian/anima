//! The device host a stored secret was entered for, sealed inside the value with the secret
//! (SEC-011), and the rule that opens it only for that host. Pure.
//!
//! A row sealed with its host uses the context `credentials.secrets.host:<device_id>:<kind>`,
//! and its plaintext is the frame: the host's length as 2 bytes big-endian, the host text, then
//! the secret. A row sealed before SEC-011 uses the context `credentials.secrets:<device_id>:
//! <kind>`, holds only the secret, and stands for the host migration 0220 recorded in its
//! `legacy_host` column. The two contexts never open each other's rows, so a database writer
//! cannot make a host-bound row read as an old one, nor the reverse.
//!
//! The host is the device's configured host text, compared without letter case; never a
//! resolved address, so a name whose address changes keeps its secrets.

use zeroize::Zeroizing;

use crate::ids::DeviceId;

use super::{Error, Sealed, Secret, Vault, store::StoredSecret};

/// The associated data of a row sealed with its host: device and kind, as the old context, under
/// another prefix.
pub(super) fn context(device: DeviceId, kind: &str) -> Vec<u8> {
    format!("credentials.secrets.host:{device}:{kind}").into_bytes()
}

/// The plaintext to seal: the host's length, the host, then the secret.
///
/// # Errors
/// [`Error::Malformed`] for a host longer than 65,535 bytes, which no device host is.
pub(super) fn frame(host: &str, secret: &Secret) -> Result<Secret, Error> {
    let length = u16::try_from(host.len()).map_err(|_| Error::Malformed)?;
    let mut plain = Zeroizing::new(Vec::with_capacity(2 + host.len() + secret.expose().len()));
    plain.extend_from_slice(&length.to_be_bytes());
    plain.extend_from_slice(host.as_bytes());
    plain.extend_from_slice(secret.expose());
    Ok(Secret::new(std::mem::take(&mut *plain)))
}

/// The host and the secret of an opened [`frame`]; the length is checked, never trusted.
///
/// # Errors
/// [`Error::Malformed`] when the length runs past the plaintext or the host is not UTF-8.
pub(super) fn unframe(plain: &[u8]) -> Result<(&str, Secret), Error> {
    let (length, rest) = plain.split_first_chunk::<2>().ok_or(Error::Malformed)?;
    let (host, secret) = rest
        .split_at_checked(usize::from(u16::from_be_bytes(*length)))
        .ok_or(Error::Malformed)?;
    let host = std::str::from_utf8(host).map_err(|_| Error::Malformed)?;
    Ok((host, Secret::new(secret.to_vec())))
}

/// Whether a secret entered for `bound` may go to `host`: the same text, letter case aside.
pub(super) fn permits(bound: &str, host: &str) -> bool {
    bound.eq_ignore_ascii_case(host)
}

/// Seals `secret` for the device's row of `kind`, bound to `host`.
///
/// # Errors
/// [`Error::Random`] when the OS random source fails; [`Error::Malformed`] as [`frame`].
pub(super) fn seal(
    vault: &Vault,
    (device, host): (DeviceId, &str),
    kind: &str,
    secret: &Secret,
) -> Result<Sealed, Error> {
    vault.seal(&frame(host, secret)?, &context(device, kind))
}

/// Opens a stored row for a connection to `host`. `legacy` is the row's context from before
/// SEC-011, used only when the row has a `legacy_host`.
///
/// # Errors
/// [`Error::WrongKey`], [`Error::Decrypt`] or [`Error::Malformed`] when the row does not open;
/// [`Error::EnteredForOtherHost`] when it was entered for another host. The opened secret is
/// dropped, its bytes zeroed, before a refusal returns.
pub(super) fn open(
    vault: &Vault,
    row: StoredSecret,
    (device, host): (DeviceId, &str),
    legacy: &[u8],
) -> Result<Secret, Error> {
    let sealed = Sealed::from_parts(&row.nonce, row.ciphertext, row.key_id)?;
    let (secret, permitted) = if let Some(bound) = row.legacy_host {
        let secret = vault.open(&sealed, legacy)?;
        (secret, permits(&bound, host))
    } else {
        let plain = vault.open(&sealed, &context(device, &row.kind))?;
        let (bound, secret) = unframe(plain.expose())?;
        (secret, permits(bound, host))
    };
    if permitted {
        Ok(secret)
    } else {
        Err(Error::EnteredForOtherHost)
    }
}
