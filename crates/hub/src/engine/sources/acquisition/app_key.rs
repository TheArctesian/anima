//! The operator's App key: read and checked, and the short token it signs (SRC-002, PRJ-018,
//! K-E1 Section 7). Pure: the time and the random source are parameters.
//!
//! The PEM is parsed with `rustls-pki-types` and the token is an RS256 JWT signed with `ring`.
//! Nothing here is `Debug` or logged: a refusal names the field, never the value.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use ring::{
    rand::SecureRandom,
    signature::{RSA_PKCS1_SHA256, RsaKeyPair},
};
use rustls_pki_types::{PrivateKeyDer, pem::PemObject as _};
use zeroize::Zeroizing;

/// The longest PEM text the hub reads.
pub const MAX_PEM_BYTES: usize = 16 * 1024;
/// The smallest RSA key the hub signs with.
const MIN_KEY_BITS: usize = 2048;
/// The longest App ID or installation ID, in digits.
const MAX_ID_DIGITS: usize = 20;
/// How far back the token's `iat` is set, for a source whose clock is behind.
const ISSUED_BEFORE: TimeDelta = TimeDelta::seconds(60);
/// How long the token is valid after `now`; GitHub accepts at most ten minutes.
const VALID_FOR: TimeDelta = TimeDelta::seconds(540);

/// Why an App key was refused. Names the field, never its value (LOG-007, PRJ-018).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum KeyRefusal {
    #[error("the App ID must be digits only")]
    AppId,
    #[error("the installation ID must be digits only")]
    InstallationId,
    #[error("the private key is longer than 16 KiB")]
    TooLong,
    #[error("the private key is not an RSA private key in PEM form")]
    NotRsaPem,
    #[error("the private key is shorter than 2048 bits")]
    TooShort,
}

impl KeyRefusal {
    /// The `GithubAppInput` field the operator must fix.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::AppId => "appId",
            Self::InstallationId => "installationId",
            Self::TooLong | Self::NotRsaPem | Self::TooShort => "privateKey",
        }
    }
}

/// The signing failed: the random source gave nothing. Fixed words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the App token could not be signed")]
pub struct SignFailed;

/// A checked App key, ready to sign. No `Debug` and no `Clone`: it holds the private key.
pub struct AppKey {
    app_id: u64,
    installation_id: String,
    pair: RsaKeyPair,
}

/// Digits only, 1 to 20 of them.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_ID_DIGITS && text.bytes().all(|b| b.is_ascii_digit())
}

impl AppKey {
    /// Reads and checks the three parts of a GitHub App key.
    ///
    /// # Errors
    /// [`KeyRefusal`] for the first part that is not right.
    pub fn read(app_id: &str, installation_id: &str, pem: &[u8]) -> Result<Self, KeyRefusal> {
        let app_id = Some(app_id)
            .filter(|id| digits(id))
            .and_then(|id| id.parse::<u64>().ok())
            .filter(|id| *id > 0)
            .ok_or(KeyRefusal::AppId)?;
        if !digits(installation_id) {
            return Err(KeyRefusal::InstallationId);
        }
        if pem.len() > MAX_PEM_BYTES {
            return Err(KeyRefusal::TooLong);
        }
        let der =
            Zeroizing::new(PrivateKeyDer::from_pem_slice(pem).map_err(|_| KeyRefusal::NotRsaPem)?);
        let pair = match &*der {
            PrivateKeyDer::Pkcs1(key) => RsaKeyPair::from_der(key.secret_pkcs1_der()),
            PrivateKeyDer::Pkcs8(key) => RsaKeyPair::from_pkcs8(key.secret_pkcs8_der()),
            _ => return Err(KeyRefusal::NotRsaPem),
        }
        // ring refuses a key under 2048 bits itself; its reason is a fixed word.
        .map_err(|rejected| {
            if rejected.to_string() == "TooSmall" {
                KeyRefusal::TooShort
            } else {
                KeyRefusal::NotRsaPem
            }
        })?;
        if pair.public().modulus_len() * 8 < MIN_KEY_BITS {
            return Err(KeyRefusal::TooShort);
        }
        Ok(Self {
            app_id,
            installation_id: installation_id.to_owned(),
            pair,
        })
    }

    /// `<App ID>/<installation ID>`: the account's instance key.
    #[must_use]
    pub fn instance_key(&self) -> String {
        format!("{}/{}", self.app_id, self.installation_id)
    }

    #[must_use]
    pub fn installation_id(&self) -> &str {
        &self.installation_id
    }

    /// The RS256 JWT that asks the source for an installation token: `iss` is the App ID,
    /// `iat` a minute before `now`, `exp` nine minutes after.
    ///
    /// # Errors
    /// [`SignFailed`] when the random source fails.
    pub fn token(
        &self,
        now: DateTime<Utc>,
        random: &(dyn SecureRandom + Sync),
    ) -> Result<Zeroizing<String>, SignFailed> {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let claims = format!(
            r#"{{"iat":{},"exp":{},"iss":{}}}"#,
            (now - ISSUED_BEFORE).timestamp(),
            (now + VALID_FOR).timestamp(),
            self.app_id
        );
        // Room for an 8192-bit signature, so the buffer never grows and leaves no copy.
        let mut jwt = Zeroizing::new(String::with_capacity(2048));
        jwt.push_str(&header);
        jwt.push('.');
        jwt.push_str(&URL_SAFE_NO_PAD.encode(claims));
        let mut signature = vec![0; self.pair.public().modulus_len()];
        self.pair
            .sign(&RSA_PKCS1_SHA256, random, jwt.as_bytes(), &mut signature)
            .map_err(|_| SignFailed)?;
        jwt.push('.');
        jwt.push_str(&URL_SAFE_NO_PAD.encode(signature));
        Ok(jwt)
    }

    /// The public key, for tests that check a signature.
    #[cfg(test)]
    #[must_use]
    pub fn public_key(&self) -> &[u8] {
        self.pair.public().as_ref()
    }
}
