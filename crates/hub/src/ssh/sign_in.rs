//! The sign-in with a stored credential (OD-9, D-5): one try of the password or the private
//! key, and keyboard-interactive answered with the password only when the device offers that
//! and not `password`. Nothing here logs; no error holds the device's text or a secret.

use std::sync::Arc;

use russh::{
    MethodKind,
    client::{AuthResult, Handle, KeyboardInteractiveAuthResponse},
    keys::{PrivateKeyWithHashAlg, decode_secret_key},
};

use super::{Observer, SshError};
use crate::credentials::{Secret, SshSecret};

/// The most keyboard-interactive rounds the hub answers before it counts the sign-in as
/// refused, so a device cannot keep it answering prompts (D-5).
const PROMPT_ROUNDS: usize = 4;
/// The most prompts the hub answers in one round; a round with more is a refusal, so a
/// device cannot have the password copied without end (D-5).
const PROMPTS_PER_ROUND: usize = 4;

/// Signs in as `username` with `secret`: a password (answering keyboard-interactive prompts
/// with it when the device offers that and not `password`, D-5), or a private key with its
/// passphrase. russh copies what it sends into buffers of its own, which this cannot wipe.
///
/// # Errors
/// [`SshError::AuthFailed`] when the device refuses, [`SshError::UnreadableKey`] when the key
/// or passphrase does not decode, [`SshError::Protocol`] when the connection fails meanwhile.
pub(crate) async fn authenticate(
    handle: &mut Handle<Observer>,
    username: &str,
    secret: &SshSecret,
) -> Result<(), SshError> {
    let result = match secret {
        SshSecret::Password(password) => {
            let password =
                std::str::from_utf8(password.expose()).map_err(|_| SshError::AuthFailed)?;
            sign_in_with_password(handle, username, password).await?
        }
        SshSecret::PrivateKey { key, passphrase } => {
            let key = private_key(key.expose(), passphrase.as_ref().map(Secret::expose))?;
            let hash = if key.algorithm().is_rsa() {
                handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|_| SshError::Protocol)?
                    .flatten()
            } else {
                None
            };
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            handle
                .authenticate_publickey(username, key)
                .await
                .map_err(|_| SshError::Protocol)?
        }
    };
    match result {
        AuthResult::Success => Ok(()),
        AuthResult::Failure { .. } => Err(SshError::AuthFailed),
    }
}

/// The `password` method, then, when the device refused it and offers `keyboard-interactive`
/// but not `password`, the password as the answer to each of its prompts (D-5).
async fn sign_in_with_password(
    handle: &mut Handle<Observer>,
    username: &str,
    password: &str,
) -> Result<AuthResult, SshError> {
    let result = handle
        .authenticate_password(username, password)
        .await
        .map_err(|_| SshError::Protocol)?;
    let AuthResult::Failure {
        remaining_methods, ..
    } = &result
    else {
        return Ok(result);
    };
    let prompts_only = remaining_methods.contains(&MethodKind::KeyboardInteractive)
        && !remaining_methods.contains(&MethodKind::Password);
    if !prompts_only {
        return Ok(result);
    }
    answer_prompts(handle, username, password).await
}

/// Answers every keyboard-interactive prompt with the password, for at most
/// [`PROMPT_ROUNDS`] rounds of at most [`PROMPTS_PER_ROUND`] prompts. The prompts are the
/// device's text: never read, logged or kept.
async fn answer_prompts(
    handle: &mut Handle<Observer>,
    username: &str,
    password: &str,
) -> Result<AuthResult, SshError> {
    let mut reply = handle
        .authenticate_keyboard_interactive_start(username, None)
        .await
        .map_err(|_| SshError::Protocol)?;
    for _ in 0..PROMPT_ROUNDS {
        let KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } = reply else {
            break;
        };
        let prompts = prompts.len();
        if prompts > PROMPTS_PER_ROUND {
            return Err(SshError::AuthFailed);
        }
        let answers = vec![password.to_owned(); prompts];
        reply = handle
            .authenticate_keyboard_interactive_respond(answers)
            .await
            .map_err(|_| SshError::Protocol)?;
    }
    // The reply to the last allowed answer counts; another round of prompts does not.
    match reply {
        KeyboardInteractiveAuthResponse::Success => Ok(AuthResult::Success),
        _ => Err(SshError::AuthFailed),
    }
}

/// Decodes an `OpenSSH`, PEM or `PuTTY` private key; the decoder's error is dropped unread.
fn private_key(key: &[u8], passphrase: Option<&[u8]>) -> Result<russh::keys::PrivateKey, SshError> {
    let key = std::str::from_utf8(key).map_err(|_| SshError::UnreadableKey)?;
    let passphrase = passphrase
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|_| SshError::UnreadableKey)?;
    decode_secret_key(key, passphrase).map_err(|_| SshError::UnreadableKey)
}
