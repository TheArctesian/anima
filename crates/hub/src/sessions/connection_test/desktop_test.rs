//! The connection test for RDP and VNC (DEV-013, SEC-007): whether the device answers on the
//! protocol's port, and for RDP the server certificate, kept for `acceptServerCertificate`.
//!
//! The hub never signs in during an RDP or VNC test: no credential, given or stored, leaves
//! the hub. RDP stops after the TLS handshake, before `CredSSP`; VNC after the version banner,
//! before the security handshake.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    rdp::{Reply, read_server_certificate},
    reach::ReadError,
    ssh_test::{ConnectionTest, refused_host},
    vnc::read_banner,
    x224::{PROTOCOL_RDP, TLS_PROTOCOLS},
};
use crate::{
    devices::RdpSecurity,
    ids::DeviceId,
    sessions::{Desktop, Error, find_device, live::Live},
    ssh::{Target, probe::PROBE_TIMEOUT},
    trust::{self, Endpoint, Observation, PresentedCertificate, ServerCertificate},
};

/// The port and, for RDP, the security to test with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum DesktopSettings {
    Rdp { port: u16, security: RdpSecurity },
    Vnc { port: u16 },
}

/// What to test: a saved device with its saved settings, or the form's values. No
/// certificate can be accepted for unsaved values yet, so any certificate read is new.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum DesktopSubject {
    Device {
        id: DeviceId,
        protocol: Desktop,
    },
    Unsaved {
        host: String,
        settings: DesktopSettings,
    },
}

pub(in crate::sessions) const NEW_CERTIFICATE: &str = "The hub has not accepted this server \
     certificate yet. Compare its fingerprint with the device's own, then accept it. An unsaved \
     device must be saved first. The hub does not sign in during a test.";
pub(in crate::sessions) const CHANGED_CERTIFICATE: &str = "The device presented a different \
     server certificate from the one accepted. Accept the new certificate only if you know why \
     it changed. The hub does not sign in during a test.";
pub(in crate::sessions) const ACCEPTED_CERTIFICATE: &str = "The server certificate is the \
     accepted one. The hub does not sign in during a test, so the credential was not checked.";
pub(in crate::sessions) const STANDARD_ONLY: &str = "The device offers only standard RDP \
     security, which has no certificate to check. The hub does not sign in during a test.";
pub(in crate::sessions) const NEGOTIATION_REFUSED: &str = "The device refused the RDP security \
     the hub asked for. Check the security setting. The hub does not sign in during a test.";
pub(in crate::sessions) const NO_TLS: &str =
    "The device answered on RDP but did not complete a TLS handshake.";
pub(in crate::sessions) const VNC_ANSWERED: &str = "The device answered as a VNC server. The \
     hub does not sign in during a test; no credential was sent.";
pub(in crate::sessions) const UNRESOLVED: &str = "The host name does not resolve.";
pub(in crate::sessions) const UNREACHABLE: &str = "The device does not answer on that port.";
pub(in crate::sessions) const TIMED_OUT: &str = "The device did not finish within 8 seconds.";
pub(in crate::sessions) const NOT_RDP: &str = "The device did not answer as an RDP server.";
pub(in crate::sessions) const NOT_VNC: &str = "The device did not answer as a VNC server.";

/// What an RDP read found, once a certificate is recorded.
enum Seen {
    Certificate(Observation<PresentedCertificate>),
    Standard,
    Refused,
}

/// Tests `subject` at `at`. A certificate the server presents is kept 15 minutes for
/// `acceptServerCertificate`, whether or not the device ignores certificates.
///
/// # Errors
/// `NotFound` for an unknown device; `Validation` on `host` when it resolves to a refused
/// address (R-032); `Database` or `Devices` when a dependency fails. A device that cannot be
/// reached is a result, not an error.
pub(in crate::sessions) async fn test_desktop(
    db: &PgPool,
    live: &Live,
    subject: DesktopSubject,
    at: DateTime<Utc>,
) -> Result<ConnectionTest, Error> {
    let (host, settings, accepted) = plan(db, subject).await?;
    match settings {
        DesktopSettings::Rdp { port, security } => {
            let requested = if security == RdpSecurity::Rdp {
                PROTOCOL_RDP
            } else {
                TLS_PROTOCOLS
            };
            let target = target(live, &host, port);
            let read = read_server_certificate(target, requested, PROBE_TIMEOUT).await;
            let seen = read.map(|reply| match reply {
                Reply::Certificate(certificate, _) => {
                    let endpoint = Endpoint::new(&host, port);
                    Seen::Certificate(live.sightings().observe_certificate(
                        endpoint,
                        certificate,
                        at,
                    ))
                }
                Reply::Standard => Seen::Standard,
                Reply::Refused => Seen::Refused,
            });
            rdp_outcome(accepted.as_ref(), seen)
        }
        DesktopSettings::Vnc { port } => {
            vnc_outcome(read_banner(target(live, &host, port), PROBE_TIMEOUT).await)
        }
    }
}

fn target<'a>(live: &'a Live, host: &'a str, port: u16) -> Target<'a> {
    Target {
        host,
        port,
        policy: live.policy(),
    }
}

/// The host, the settings and, for a saved device tested with RDP, its accepted certificate.
async fn plan(
    db: &PgPool,
    subject: DesktopSubject,
) -> Result<(String, DesktopSettings, Option<ServerCertificate>), Error> {
    let (id, protocol) = match subject {
        DesktopSubject::Unsaved { host, settings } => return Ok((host, settings, None)),
        DesktopSubject::Device { id, protocol } => (id, protocol),
    };
    let device = find_device(db, id).await?;
    Ok(match protocol {
        Desktop::Rdp => {
            let accepted = trust::server_certificates(db, &[id]).await?.remove(&id);
            let settings = DesktopSettings::Rdp {
                port: device.rdp.port,
                security: device.rdp.security,
            };
            (device.host, settings, accepted)
        }
        Desktop::Vnc => (
            device.host,
            DesktopSettings::Vnc {
                port: device.vnc.port,
            },
            None,
        ),
    })
}

/// A device that answered, signed in to by nobody.
fn reached(message: &'static str) -> ConnectionTest {
    ConnectionTest {
        reachable: true,
        authenticated: false,
        host_key: None,
        host_key_changed: false,
        server_certificate: None,
        certificate_changed: false,
        certificate_matched: false,
        message,
    }
}

/// The result for a read that failed, with `unexpected` for a reply of another protocol; a
/// refused target is the operator's `Validation` error on `host` (R-032).
fn unreached(
    read: Result<ConnectionTest, ReadError>,
    unexpected: &'static str,
) -> Result<ConnectionTest, Error> {
    let message = match read {
        Ok(test) => return Ok(test),
        Err(ReadError::RefusedTarget) => return Err(Error::Validation(vec![refused_host()])),
        Err(ReadError::Tls) => return Ok(reached(NO_TLS)),
        Err(ReadError::Unresolved) => UNRESOLVED,
        Err(ReadError::Unreachable) => UNREACHABLE,
        Err(ReadError::Timeout) => TIMED_OUT,
        Err(ReadError::Unexpected) => unexpected,
    };
    Ok(ConnectionTest {
        reachable: false,
        ..reached(message)
    })
}

/// `serverCertificate` is set when none is accepted (`certificateChanged` false) or the server
/// presented another (`certificateChanged` true), and null when it presented the accepted one,
/// the only case `certificateMatched` is true.
fn rdp_outcome(
    accepted: Option<&ServerCertificate>,
    seen: Result<Seen, ReadError>,
) -> Result<ConnectionTest, Error> {
    let read = seen.map(|seen| match seen {
        Seen::Standard => reached(STANDARD_ONLY),
        Seen::Refused => reached(NEGOTIATION_REFUSED),
        Seen::Certificate(seen) => {
            let seen = ServerCertificate::from(seen);
            match accepted {
                Some(accepted) if accepted.fingerprint_sha256 == seen.fingerprint_sha256 => {
                    ConnectionTest {
                        certificate_matched: true,
                        ..reached(ACCEPTED_CERTIFICATE)
                    }
                }
                accepted => ConnectionTest {
                    server_certificate: Some(seen),
                    certificate_changed: accepted.is_some(),
                    ..reached(if accepted.is_some() {
                        CHANGED_CERTIFICATE
                    } else {
                        NEW_CERTIFICATE
                    })
                },
            }
        }
    });
    unreached(read, NOT_RDP)
}

/// The VNC result: a banner is an answer, nothing more; VNC has no certificate.
fn vnc_outcome(read: Result<(), ReadError>) -> Result<ConnectionTest, Error> {
    unreached(read.map(|()| reached(VNC_ANSWERED)), NOT_VNC)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{
        ACCEPTED_CERTIFICATE, CHANGED_CERTIFICATE, NEGOTIATION_REFUSED, NEW_CERTIFICATE, NO_TLS,
        STANDARD_ONLY, Seen, UNREACHABLE, VNC_ANSWERED, rdp_outcome, vnc_outcome,
    };
    use crate::{
        sessions::connection_test::ReadError,
        trust::{Observation, PresentedCertificate, ServerCertificate},
    };

    fn observed(der: &[u8]) -> Observation<PresentedCertificate> {
        Observation {
            presented: PresentedCertificate::from_der(der),
            first_seen_at: DateTime::<Utc>::UNIX_EPOCH,
            last_seen_at: DateTime::<Utc>::UNIX_EPOCH,
        }
    }

    /// `certificateMatched` is true only for a certificate equal to the pin: a null
    /// `serverCertificate` alone (standard security, a refusal, no TLS) proves nothing.
    #[test]
    fn sec_007_only_the_accepted_certificate_is_matched() {
        let accepted = ServerCertificate::from(observed(b"accepted"));
        let same = || Ok(Seen::Certificate(observed(b"accepted")));
        let (pinned, other) = (Some(&accepted), Ok(Seen::Certificate(observed(b"other"))));
        let cases = [
            ("matched", pinned, same(), ACCEPTED_CERTIFICATE),
            ("first certificate", None, same(), NEW_CERTIFICATE),
            ("changed", pinned, other, CHANGED_CERTIFICATE),
            ("STANDARD_ONLY", pinned, Ok(Seen::Standard), STANDARD_ONLY),
            (
                "NEGOTIATION_REFUSED",
                pinned,
                Ok(Seen::Refused),
                NEGOTIATION_REFUSED,
            ),
            ("NO_TLS", pinned, Err(ReadError::Tls), NO_TLS),
            (
                "unreachable",
                pinned,
                Err(ReadError::Unreachable),
                UNREACHABLE,
            ),
        ];
        for (case, pinned, read, message) in cases {
            let test = rdp_outcome(pinned, read).unwrap_or_else(|_| panic!("{case}"));
            assert_eq!(test.message, message, "{case}");
            assert_eq!(test.certificate_matched, case == "matched", "{case}");
        }
    }

    #[test]
    fn sec_007_a_vnc_test_never_matches_a_certificate() {
        let cases = [
            ("answered", Ok(())),
            ("unreachable", Err(ReadError::Unreachable)),
        ];
        for (case, read) in cases {
            let test = vnc_outcome(read).unwrap_or_else(|_| panic!("{case}"));
            assert!(!test.certificate_matched, "{case}");
        }
        assert_eq!(
            vnc_outcome(Ok(())).map(|test| test.message).ok(),
            Some(VNC_ANSWERED)
        );
    }
}
