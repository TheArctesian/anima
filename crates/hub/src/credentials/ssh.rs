//! SSH secrets and their pure mapping to stored rows: one sealed row per part (SEC-003).

use crate::ids::DeviceId;

use super::{Error, Secret};

/// The SSH credential of a device: a password, or a private key with an optional passphrase.
///
/// Implements no `Debug`, `Display`, `Serialize` or `Clone` (LOG-007, SEC-005).
pub enum SshSecret {
    Password(Secret),
    PrivateKey {
        key: Secret,
        passphrase: Option<Secret>,
    },
}

/// Which part of an SSH secret a row holds; the `kind` column of `credentials.secrets`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SshKind {
    Password,
    PrivateKey,
    Passphrase,
}

impl SshKind {
    /// Every SSH `kind` column value. SSH reads, checks and deletes match only these, so a row
    /// of another kind is never taken for part of an SSH secret.
    pub(super) const SQL_KINDS: [&'static str; 3] = [
        Self::Password.as_sql(),
        Self::PrivateKey.as_sql(),
        Self::Passphrase.as_sql(),
    ];

    /// The `kind` column value.
    pub(super) const fn as_sql(self) -> &'static str {
        match self {
            Self::Password => "ssh_password",
            Self::PrivateKey => "ssh_private_key",
            Self::Passphrase => "ssh_passphrase",
        }
    }

    /// Parses a `kind` column value.
    ///
    /// # Errors
    ///
    /// [`Error::Malformed`] for a value that names no SSH kind.
    pub(super) fn from_sql(text: &str) -> Result<Self, Error> {
        match text {
            "ssh_password" => Ok(Self::Password),
            "ssh_private_key" => Ok(Self::PrivateKey),
            "ssh_passphrase" => Ok(Self::Passphrase),
            _ => Err(Error::Malformed),
        }
    }
}

/// The associated data that binds a sealed value to its device and kind, so a value moved to
/// another row does not open:
/// `credentials.secrets:<device_id>:<kind>`, the device id as hyphenated lowercase UUID text.
pub(super) fn context(device: DeviceId, kind: SshKind) -> Vec<u8> {
    format!("credentials.secrets:{device}:{}", kind.as_sql()).into_bytes()
}

impl SshSecret {
    /// Splits the credential into one secret per stored row.
    pub(super) fn into_parts(self) -> Vec<(SshKind, Secret)> {
        match self {
            Self::Password(password) => vec![(SshKind::Password, password)],
            Self::PrivateKey { key, passphrase } => {
                let mut parts = vec![(SshKind::PrivateKey, key)];
                parts.extend(passphrase.map(|p| (SshKind::Passphrase, p)));
                parts
            }
        }
    }

    /// Rebuilds the credential from its rows.
    ///
    /// # Errors
    ///
    /// [`Error::Malformed`] for a row set no [`SshSecret`] produces: no rows, a kind twice,
    /// a password together with a key or passphrase, or a passphrase without a key.
    pub(super) fn from_parts(
        parts: impl IntoIterator<Item = (SshKind, Secret)>,
    ) -> Result<Self, Error> {
        let (mut password, mut key, mut passphrase) = (None, None, None);
        for (kind, secret) in parts {
            let slot = match kind {
                SshKind::Password => &mut password,
                SshKind::PrivateKey => &mut key,
                SshKind::Passphrase => &mut passphrase,
            };
            if slot.replace(secret).is_some() {
                return Err(Error::Malformed);
            }
        }
        match (password, key, passphrase) {
            (Some(password), None, None) => Ok(Self::Password(password)),
            (None, Some(key), passphrase) => Ok(Self::PrivateKey { key, passphrase }),
            _ => Err(Error::Malformed),
        }
    }
}
