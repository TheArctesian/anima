//! The connection test (DEV-013, K4 §6, D-5): reads the host's key, keeps what it saw for
//! `acceptHostKey`, and signs in only when the key is the accepted one, so a man in the
//! middle never receives a credential.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::observations::{Endpoint, Observation};
use crate::{
    credentials::{self, SshSecret, Vault},
    ids::DeviceId,
    sessions::{
        Error, FieldError, HostKey, find_device, host_key,
        live::Live,
        probe::{Login, PROBE_TIMEOUT, ProbeError, SignIn, Target, read_host_key, sign_in},
    },
};

/// What to test.
pub(in crate::sessions) enum TestSubject {
    /// A saved device. `credential`, when given, is used for this test instead of the
    /// stored secret and is not stored.
    Device {
        id: DeviceId,
        credential: Option<SshSecret>,
    },
    /// Values from the device form. No key can be accepted for them yet, so the test reads
    /// the host key and sends no credential.
    Unsaved(Unsaved),
}

/// A host and SSH port typed into the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct Unsaved {
    pub(in crate::sessions) host: String,
    pub(in crate::sessions) port: u16,
}

/// A test and the time it starts, which stamps what it sees (CQ-021).
pub(in crate::sessions) struct TestRequest {
    pub(in crate::sessions) subject: TestSubject,
    pub(in crate::sessions) at: DateTime<Utc>,
}

/// K1 `ConnectionTestResult`. `message` is one of this module's fixed texts, never the
/// host's banner or error text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct ConnectionTest {
    pub(in crate::sessions) reachable: bool,
    pub(in crate::sessions) authenticated: bool,
    /// Set when no key is accepted or the host presented another key.
    pub(in crate::sessions) host_key: Option<HostKey>,
    pub(in crate::sessions) host_key_changed: bool,
    pub(in crate::sessions) message: &'static str,
}

pub(in crate::sessions) const SIGNED_IN: &str = "Connected and signed in.";
pub(in crate::sessions) const NEW_KEY: &str = "The hub has not accepted this host key yet. \
     Compare its fingerprint with the device's own, then accept it to sign in. An unsaved \
     device must be saved first.";
pub(in crate::sessions) const CHANGED_KEY: &str = "The device presented a different host key \
     from the one accepted. No credential was sent. Accept the new key only if you know why \
     it changed.";
pub(in crate::sessions) const NO_CREDENTIAL: &str =
    "The host key is the accepted one, but the hub holds no credential for this device.";
pub(in crate::sessions) const AUTH_FAILED: &str = "The device refused the credential.";
pub(in crate::sessions) const UNREADABLE_KEY: &str =
    "The private key or its passphrase cannot be read.";
pub(in crate::sessions) const UNRESOLVED: &str = "The host name does not resolve.";
pub(in crate::sessions) const UNREACHABLE: &str = "The device does not answer on its SSH port.";
pub(in crate::sessions) const TIMED_OUT: &str = "The device did not finish within 10 seconds.";
pub(in crate::sessions) const NOT_SSH: &str = "The device did not complete an SSH key exchange.";

/// Where the test goes and what it may send.
struct Plan {
    host: String,
    port: u16,
    accepted: Option<HostKey>,
    /// The user name and secret, read only when a key is accepted.
    login: Option<(String, SshSecret)>,
}

/// What the probe saw.
enum Seen {
    SignedIn,
    Key(Observation),
}

/// Runs the connection test (DEV-013). Every key the host presents is kept for 15 minutes
/// for `acceptHostKey` (D-5). A credential, given or stored, is read and sent only when the
/// device has an accepted key and the host presents exactly that key.
///
/// # Errors
/// `NotFound` for an unknown device; `Validation` on `host` when it resolves to a refused
/// address (R-032); `Database`, `Devices` or `Credentials` when a dependency fails. A host
/// that cannot be reached or signed in to is a result, not an error.
pub(in crate::sessions) async fn test_connection(
    db: &PgPool,
    vault: &Vault,
    live: &Live,
    request: TestRequest,
) -> Result<ConnectionTest, Error> {
    let plan = plan(db, vault, request.subject).await?;
    let target = Target {
        host: &plan.host,
        port: plan.port,
        rule: live.target_rule(),
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
            .map(|read| SignIn::OtherKey(read.key)),
    };
    let endpoint = Endpoint::new(&plan.host, plan.port);
    let seen = match probed {
        Ok(SignIn::SignedIn) => Ok(Seen::SignedIn),
        Ok(SignIn::OtherKey(key)) => Ok(Seen::Key(live.observe(endpoint, key, request.at))),
        Err(ProbeError::RefusedTarget) => return Err(Error::Validation(vec![refused_host()])),
        Err(error) => Err(error),
    };
    Ok(outcome(plan.accepted.as_ref(), seen))
}

async fn plan(db: &PgPool, vault: &Vault, subject: TestSubject) -> Result<Plan, Error> {
    let (id, credential) = match subject {
        TestSubject::Unsaved(Unsaved { host, port }) => {
            return Ok(Plan {
                host,
                port,
                accepted: None,
                login: None,
            });
        }
        TestSubject::Device { id, credential } => (id, credential),
    };
    let device = find_device(db, id).await?;
    let accepted = host_key(db, id).await?;
    let secret = match (&accepted, credential) {
        (None, _) => None,
        (Some(_), Some(credential)) => Some(credential),
        (Some(_), None) => credentials::load_ssh(db, vault, id)
            .await
            .map_err(Error::Credentials)?,
    };
    Ok(Plan {
        host: device.host,
        port: device.ssh.port,
        accepted,
        login: secret.map(|secret| (device.ssh.username, secret)),
    })
}

fn refused_host() -> FieldError {
    FieldError::new(
        "host",
        "The host resolves to a loopback, link-local, multicast or unspecified address. \
         Enter the address the device has on its network.",
    )
}

/// The result K1 describes for what the probe saw, given the accepted key.
fn outcome(accepted: Option<&HostKey>, seen: Result<Seen, ProbeError>) -> ConnectionTest {
    let reached = |message| ConnectionTest {
        reachable: true,
        authenticated: false,
        host_key: None,
        host_key_changed: false,
        message,
    };
    let unreached = |message| ConnectionTest {
        reachable: false,
        ..reached(message)
    };
    match seen {
        Ok(Seen::SignedIn) => ConnectionTest {
            authenticated: true,
            ..reached(SIGNED_IN)
        },
        Ok(Seen::Key(seen)) => match accepted {
            Some(accepted) if accepted.public_key == seen.key => reached(NO_CREDENTIAL),
            accepted => ConnectionTest {
                host_key: Some(HostKey {
                    public_key: seen.key,
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
        Err(ProbeError::AuthFailed) => reached(AUTH_FAILED),
        Err(ProbeError::UnreadableKey) => reached(UNREADABLE_KEY),
        Err(ProbeError::Unresolved | ProbeError::RefusedTarget) => unreached(UNRESOLVED),
        Err(ProbeError::Unreachable) => unreached(UNREACHABLE),
        Err(ProbeError::Timeout) => unreached(TIMED_OUT),
        Err(ProbeError::Protocol) => unreached(NOT_SSH),
    }
}
