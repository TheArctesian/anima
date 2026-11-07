//! The connection test (DEV-013, K4 §6, D-5): for SSH, reads the host's key, keeps what it
//! saw for `acceptHostKey`, and signs in only when the key is the accepted one, so a man in
//! the middle never receives a credential. RDP and VNC are tested in `desktop_test`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    desktop_test::{DesktopSubject, test_desktop},
    observations::{Endpoint, Observation},
};
use crate::{
    credentials::{self, SshSecret, Vault},
    ids::DeviceId,
    sessions::{
        Error, FieldError, HostKey, ServerCertificate, find_device, host_key,
        live::Live,
        probe::{Login, PROBE_TIMEOUT, SignIn, read_host_key, sign_in},
        public_key::PublicKey,
        ssh::{SshError, Target},
    },
};

/// What to test.
pub(in crate::sessions) enum TestSubject {
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "K1 ConnectionTestResult has these four booleans"
)]
pub(in crate::sessions) struct ConnectionTest {
    pub(in crate::sessions) reachable: bool,
    pub(in crate::sessions) authenticated: bool,
    /// Set when no key is accepted or the host presented another key.
    pub(in crate::sessions) host_key: Option<HostKey>,
    pub(in crate::sessions) host_key_changed: bool,
    /// Set when no certificate is accepted or the RDP server presented another one.
    pub(in crate::sessions) server_certificate: Option<ServerCertificate>,
    pub(in crate::sessions) certificate_changed: bool,
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
    Key(Observation<PublicKey>),
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
    let plan = match request.subject {
        TestSubject::Desktop(desktop) => return test_desktop(db, live, desktop, request.at).await,
        TestSubject::Unsaved(Unsaved { host, port }) => Plan {
            host,
            port,
            accepted: None,
            login: None,
        },
        TestSubject::Device { id, credential } => plan(db, vault, id, credential).await?,
    };
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
            .map(SignIn::OtherKey),
    };
    let endpoint = Endpoint::new(&plan.host, plan.port);
    let seen = probed.map(|signed| match signed {
        SignIn::SignedIn => Seen::SignedIn,
        SignIn::OtherKey(key) => Seen::Key(live.observe(endpoint, key, request.at)),
    });
    outcome(plan.accepted.as_ref(), seen)
}

/// A saved device's plan: its host and SSH port, its accepted key, and the credential only
/// when a key is accepted.
async fn plan(
    db: &PgPool,
    vault: &Vault,
    id: DeviceId,
    credential: Option<SshSecret>,
) -> Result<Plan, Error> {
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

pub(super) fn refused_host() -> FieldError {
    FieldError::new(
        "host",
        "The host resolves to a loopback, link-local, multicast or unspecified address. \
         Enter the address the device has on its network.",
    )
}

/// The result K1 describes for what the probe saw, given the accepted key; a refused
/// target is the operator's `Validation` error on `host` (R-032).
fn outcome(
    accepted: Option<&HostKey>,
    seen: Result<Seen, SshError>,
) -> Result<ConnectionTest, Error> {
    let reached = |message| ConnectionTest {
        reachable: true,
        authenticated: false,
        host_key: None,
        host_key_changed: false,
        server_certificate: None,
        certificate_changed: false,
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
            Some(accepted) if accepted.public_key == seen.presented => reached(NO_CREDENTIAL),
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
