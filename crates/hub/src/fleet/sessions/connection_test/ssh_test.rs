//! The connection test (DEV-013, K4 §6): for SSH, reads the host's key, keeps what it
//! saw for `acceptHostKey` (`trust::Sightings`), and signs in only when the key is the
//! accepted one, so a man in the middle never receives a credential. RDP and VNC are tested
//! in `desktop_test`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::desktop_test::{DesktopSubject, test_desktop};
use crate::{
    fleet::{
        credentials::{self, SshSecret, Vault},
        sessions::{Error, FieldError, find_device, live::Live},
        ssh::{
            SshError, Target,
            probe::{Login, PROBE_TIMEOUT, SignIn, read_host_key, sign_in},
        },
        trust::{self, Endpoint, HostKey, Observation, PublicKey, ServerCertificate},
    },
    ids::DeviceId,
};

/// What to test.
pub(in crate::fleet::sessions) enum TestSubject {
    /// A saved device over SSH. `credential`, when given, is used for this test instead of the
    /// stored secret and is not stored.
    Device {
        id: DeviceId,
        credential: Option<SshSecret>,
    },
    /// SSH values from the device form. No key can be accepted for them yet, so the test
    /// reads the host key and sends no credential.
    Unsaved(Unsaved),
    /// RDP or VNC, which never sends a credential.
    Desktop(DesktopSubject),
}

/// A host and SSH port typed into the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::sessions) struct Unsaved {
    pub(in crate::fleet::sessions) host: String,
    pub(in crate::fleet::sessions) port: u16,
}

/// A test and the time it starts, which stamps what it sees (CQ-021).
pub(in crate::fleet::sessions) struct TestRequest {
    pub(in crate::fleet::sessions) subject: TestSubject,
    pub(in crate::fleet::sessions) at: DateTime<Utc>,
}

/// K1 `ConnectionTestResult`. `message` is one of this module's fixed texts, never the
/// host's banner or error text.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "K1 ConnectionTestResult has these five booleans"
)]
pub(in crate::fleet::sessions) struct ConnectionTest {
    pub(in crate::fleet::sessions) reachable: bool,
    pub(in crate::fleet::sessions) authenticated: bool,
    /// Set when no key is accepted or the host presented another key.
    pub(in crate::fleet::sessions) host_key: Option<HostKey>,
    pub(in crate::fleet::sessions) host_key_changed: bool,
    /// Set when no certificate is accepted or the RDP server presented another one.
    pub(in crate::fleet::sessions) server_certificate: Option<ServerCertificate>,
    pub(in crate::fleet::sessions) certificate_changed: bool,
    /// True only when the RDP server presented the accepted certificate (SEC-007).
    pub(in crate::fleet::sessions) certificate_matched: bool,
    pub(in crate::fleet::sessions) message: &'static str,
}

pub(in crate::fleet::sessions) const SIGNED_IN: &str = "Connected and signed in.";
pub(in crate::fleet::sessions) const NEW_KEY: &str = "The hub has not accepted this host key yet. \
     Compare its fingerprint with the device's own, then accept it to sign in. An unsaved \
     device must be saved first.";
pub(in crate::fleet::sessions) const CHANGED_KEY: &str = "The device presented a different host key \
     from the one accepted. No credential was sent. Accept the new key only if you know why \
     it changed.";
pub(in crate::fleet::sessions) const NO_CREDENTIAL: &str =
    "The host key is the accepted one, but the hub holds no credential for this device.";
pub(in crate::fleet::sessions) const CREDENTIAL_MOVED: &str = "The host key is the accepted one, but \
     the device's address changed after its SSH credential was entered, so no credential was \
     sent. Enter the credential again.";
pub(in crate::fleet::sessions) const AUTH_FAILED: &str = "The device refused the credential.";
pub(in crate::fleet::sessions) const UNREADABLE_KEY: &str =
    "The private key or its passphrase cannot be read.";
pub(in crate::fleet::sessions) const UNRESOLVED: &str = "The host name does not resolve.";
pub(in crate::fleet::sessions) const UNREACHABLE: &str =
    "The device does not answer on its SSH port.";
pub(in crate::fleet::sessions) const TIMED_OUT: &str =
    "The device did not finish within 10 seconds.";
pub(in crate::fleet::sessions) const NOT_SSH: &str =
    "The device did not complete an SSH key exchange.";

/// Where the test goes and what it may send.
struct Plan {
    host: String,
    port: u16,
    accepted: Option<HostKey>,
    /// The user name and secret, read only when a key is accepted.
    login: Option<(String, SshSecret)>,
    /// The stored secret was entered for another host, so none is sent (SEC-011).
    credential_moved: bool,
}

/// What the probe saw.
enum Seen {
    SignedIn,
    Key(Observation<PublicKey>),
}

/// Runs the connection test (DEV-013). Every key the host presents is kept for 15 minutes
/// for `acceptHostKey` (D-5). A credential, given or stored, is read and sent only when the
/// device has an accepted key and the host presents exactly that key.
///
/// # Errors
/// `NotFound` for an unknown device; `Validation` on `host` when it resolves to an address
/// the target rule or the hub's deny list refuses (R-030, R-032); `Database`, `Devices` or
/// `Credentials` when a dependency fails. A host that cannot be reached or signed in to is a
/// result, not an error.
pub(in crate::fleet::sessions) async fn test_connection(
    db: &PgPool,
    vault: &Vault,
    live: &Live,
    request: TestRequest,
) -> Result<ConnectionTest, Error> {
    let plan = match request.subject {
        TestSubject::Desktop(desktop) => return test_desktop(db, live, desktop, request.at).await,
        TestSubject::Unsaved(Unsaved { host, port }) => Plan {
            host,
            port,
            accepted: None,
            login: None,
            credential_moved: false,
        },
        TestSubject::Device { id, credential } => plan(db, vault, id, credential).await?,
    };
    let target = Target {
        host: &plan.host,
        port: plan.port,
        policy: live.policy(),
    };
    let probed = match (&plan.accepted, &plan.login) {
        (Some(accepted), Some((username, secret))) => {
            let login = Login {
                accepted: &accepted.public_key,
                username,
                secret,
            };
            sign_in(target, &login, PROBE_TIMEOUT).await
        }
        _ => read_host_key(target, PROBE_TIMEOUT)
            .await
            .map(SignIn::OtherKey),
    };
    let endpoint = Endpoint::new(&plan.host, plan.port);
    let seen = probed.map(|signed| match signed {
        SignIn::SignedIn => Seen::SignedIn,
        SignIn::OtherKey(key) => Seen::Key(live.sightings().observe(endpoint, key, request.at)),
    });
    outcome(plan.accepted.as_ref(), plan.credential_moved, seen)
}

/// A saved device's plan: its host and SSH port, its accepted key, and the credential only
/// when a key is accepted and, for the stored one, only when it was entered for the device's
/// host now (SEC-011).
async fn plan(
    db: &PgPool,
    vault: &Vault,
    id: DeviceId,
    credential: Option<SshSecret>,
) -> Result<Plan, Error> {
    let device = find_device(db, id).await?;
    let accepted = trust::host_key(db, id).await?;
    let stored = match (&accepted, credential) {
        (None, _) => Ok(None),
        (Some(_), Some(credential)) => Ok(Some(credential)),
        (Some(_), None) => credentials::load_ssh(db, vault, (id, &device.host)).await,
    };
    let (secret, credential_moved) = match stored {
        Err(credentials::Error::EnteredForOtherHost) => (None, true),
        stored => (stored.map_err(Error::Credentials)?, false),
    };
    Ok(Plan {
        host: device.host,
        port: device.ssh.port,
        accepted,
        login: secret.map(|secret| (device.ssh.username, secret)),
        credential_moved,
    })
}

pub(super) fn refused_host() -> FieldError {
    FieldError::new(
        "host",
        "The host resolves to a loopback, link-local, multicast or unspecified address, or to \
         one in a network the hub refuses (ANIMA_PROBE_DENY). Enter the address the device \
         has on its network.",
    )
}

/// The result K1 describes for what the probe saw, given the accepted key and whether the
/// stored credential was held back because it was entered for another host; a refused target
/// is the operator's `Validation` error on `host` (R-032).
fn outcome(
    accepted: Option<&HostKey>,
    credential_moved: bool,
    seen: Result<Seen, SshError>,
) -> Result<ConnectionTest, Error> {
    let reached = |message| ConnectionTest {
        reachable: true,
        authenticated: false,
        host_key: None,
        host_key_changed: false,
        server_certificate: None,
        certificate_changed: false,
        certificate_matched: false,
        message,
    };
    let unreached = |message| ConnectionTest {
        reachable: false,
        ..reached(message)
    };
    Ok(match seen {
        Ok(Seen::SignedIn) => ConnectionTest {
            authenticated: true,
            ..reached(SIGNED_IN)
        },
        Ok(Seen::Key(seen)) => match accepted {
            Some(accepted) if accepted.public_key == seen.presented => {
                reached(if credential_moved {
                    CREDENTIAL_MOVED
                } else {
                    NO_CREDENTIAL
                })
            }
            accepted => ConnectionTest {
                host_key: Some(HostKey {
                    public_key: seen.presented,
                    first_seen_at: seen.first_seen_at,
                }),
                host_key_changed: accepted.is_some(),
                ..reached(if accepted.is_some() {
                    CHANGED_KEY
                } else {
                    NEW_KEY
                })
            },
        },
        Err(SshError::AuthFailed) => reached(AUTH_FAILED),
        Err(SshError::UnreadableKey) => reached(UNREADABLE_KEY),
        Err(SshError::RefusedTarget) => return Err(Error::Validation(vec![refused_host()])),
        Err(SshError::Unresolved) => unreached(UNRESOLVED),
        Err(SshError::Unreachable) => unreached(UNREACHABLE),
        Err(SshError::Timeout) => unreached(TIMED_OUT),
        Err(SshError::Protocol) => unreached(NOT_SSH),
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{
        AUTH_FAILED, CHANGED_KEY, NEW_KEY, NO_CREDENTIAL, SIGNED_IN, Seen, UNREACHABLE, outcome,
    };
    use crate::fleet::{
        ssh::SshError,
        trust::{HostKey, Observation, PublicKey},
    };

    fn key(data: &[u8]) -> PublicKey {
        let mut blob = 11_u32.to_be_bytes().to_vec();
        blob.extend_from_slice(b"ssh-ed25519");
        blob.extend_from_slice(data);
        PublicKey::from_blob(blob).unwrap()
    }

    fn presented(data: &[u8]) -> Seen {
        Seen::Key(Observation {
            presented: key(data),
            first_seen_at: DateTime::<Utc>::UNIX_EPOCH,
            last_seen_at: DateTime::<Utc>::UNIX_EPOCH,
        })
    }

    /// SSH has no server certificate, so even the accepted host key is no certificate match.
    #[test]
    fn sec_007_an_ssh_test_never_matches_a_certificate() {
        let accepted = HostKey {
            public_key: key(b"accepted"),
            first_seen_at: DateTime::<Utc>::UNIX_EPOCH,
        };
        let pinned = Some(&accepted);
        let cases = [
            ("signed in", pinned, Ok(Seen::SignedIn), SIGNED_IN),
            (
                "accepted key",
                pinned,
                Ok(presented(b"accepted")),
                NO_CREDENTIAL,
            ),
            ("new key", None, Ok(presented(b"accepted")), NEW_KEY),
            ("changed key", pinned, Ok(presented(b"other")), CHANGED_KEY),
            (
                "refused credential",
                pinned,
                Err(SshError::AuthFailed),
                AUTH_FAILED,
            ),
            (
                "unreachable",
                pinned,
                Err(SshError::Unreachable),
                UNREACHABLE,
            ),
        ];
        for (case, pinned, seen, message) in cases {
            let test = outcome(pinned, false, seen).unwrap_or_else(|_| panic!("{case}"));
            assert_eq!(test.message, message, "{case}");
            assert!(!test.certificate_matched, "{case}");
        }
    }
}
