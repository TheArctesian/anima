//! An RDP session opens only on the accepted certificate, with the security the session
//! asks for, and sends the account only after the certificate is checked (SEC-007).

use anima_gateway::handshake::{Certificate, Security};
use anima_gateway::rdp::{ClipboardNotes, OpenError, Opened, Wanted, open};
use tokio::io::AsyncWriteExt as _;
use tokio::net::{TcpListener, TcpStream};

use super::device::{Device, Offered, Seen, Setup, WAIT};

const SIZE: (u16, u16) = (320, 240);
pub(super) const USERNAME: &str = "operator";
/// Made up for the tests.
pub(super) const PASSWORD: &str = "correct horse battery";
const DOMAIN: &str = "WORKGROUP";

pub(super) const TLS: Setup = Setup {
    size: SIZE,
    offered: Offered::Tls,
};
const NLA: Setup = Setup {
    size: SIZE,
    offered: Offered::Nla {
        username: USERNAME,
        password: PASSWORD,
    },
};

/// The session's wishes: the account, a screen larger than the device's, and this
/// security and certificate rule.
pub(super) fn wanted(security: Security, certificate: Certificate) -> Wanted<'static> {
    Wanted {
        username: USERNAME,
        password: PASSWORD,
        domain: Some(DOMAIN),
        security,
        certificate,
        size: (1024, 768),
    }
}

/// Opens a session on a new connection to `address`.
pub(super) async fn opened(
    address: std::net::SocketAddr,
    wanted: Wanted<'_>,
) -> Result<Opened, OpenError> {
    let device = TcpStream::connect(address).await.expect("connected");
    tokio::time::timeout(WAIT, open(device, wanted, ClipboardNotes::default()))
        .await
        .expect("the opening ends in time")
}

#[tokio::test]
async fn a_tls_sign_in_with_the_accepted_certificate_opens_at_the_devices_size() {
    let mut device = Device::start(TLS);
    let accepted = Certificate::Accepted(device.fingerprint);

    let opened = opened(device.address, wanted(Security::Tls, accepted))
        .await
        .expect("opened");

    let size = opened.result.desktop_size;
    assert_eq!((size.width, size.height), SIZE);
    let sign_in = Seen::SignIn {
        username: USERNAME.to_owned(),
        password: PASSWORD.to_owned(),
        domain: Some(DOMAIN.to_owned()),
    };
    assert_eq!(device.next().await, sign_in);
}

#[tokio::test]
async fn another_certificate_ends_the_opening_before_any_credential_is_sent() {
    let mut device = Device::start(TLS);
    let mut other = device.fingerprint;
    other[0] ^= 1;

    let refused = opened(
        device.address,
        wanted(Security::Tls, Certificate::Accepted(other)),
    )
    .await;

    assert_eq!(refused.err(), Some(OpenError::Certificate));
    assert_eq!(device.next().await, Seen::Ended);
    // The device still serves the next client after a connection that stopped at TLS.
    let accepted = Certificate::Accepted(device.fingerprint);
    let opened = opened(device.address, wanted(Security::Tls, accepted)).await;
    assert_eq!(opened.err(), None);
}

#[tokio::test]
async fn any_certificate_opens_when_the_operator_ignores_the_certificate() {
    let device = Device::start(TLS);

    let opened = opened(device.address, wanted(Security::TlsOrNla, Certificate::Any)).await;

    assert_eq!(opened.err(), None);
}

/// A server on a loopback port that sends `answer` to its first client, then closes.
async fn not_rdp(answer: &'static [u8]) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("an address");
    tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.expect("a client");
        client.write_all(answer).await.expect("answered");
    });
    address
}

#[tokio::test]
async fn a_server_that_answers_with_other_bytes_is_not_rdp() {
    let address = not_rdp(b"SSH-2.0-OpenSSH_9.8\r\n").await;

    let refused = opened(address, wanted(Security::Tls, Certificate::Any)).await;

    assert_eq!(refused.err(), Some(OpenError::NotRdp));
}

#[tokio::test]
async fn a_server_that_closes_at_once_is_not_rdp() {
    let address = not_rdp(b"").await;

    let refused = opened(address, wanted(Security::Tls, Certificate::Any)).await;

    assert_eq!(refused.err(), Some(OpenError::NotRdp));
}

#[tokio::test]
async fn nla_against_a_device_that_offers_tls_only_ends_on_the_security() {
    let device = Device::start(TLS);

    let refused = opened(device.address, wanted(Security::Nla, Certificate::Any)).await;

    assert_eq!(refused.err(), Some(OpenError::Security));
}

#[tokio::test]
async fn tls_against_a_device_that_requires_nla_ends_on_the_security() {
    let device = Device::start(NLA);

    let refused = opened(device.address, wanted(Security::Tls, Certificate::Any)).await;

    assert_eq!(refused.err(), Some(OpenError::Security));
}

#[tokio::test]
async fn nla_with_the_right_account_opens_at_the_devices_size() {
    let device = Device::start(NLA);
    let accepted = Certificate::Accepted(device.fingerprint);

    let opened = opened(device.address, wanted(Security::Nla, accepted))
        .await
        .expect("opened");

    let size = opened.result.desktop_size;
    assert_eq!((size.width, size.height), SIZE);
}

#[tokio::test]
async fn either_security_signs_in_with_nla_on_a_device_that_requires_it() {
    let device = Device::start(NLA);
    let accepted = Certificate::Accepted(device.fingerprint);

    let opened = opened(device.address, wanted(Security::TlsOrNla, accepted)).await;

    assert_eq!(opened.err(), None);
}

#[tokio::test]
async fn nla_with_a_wrong_password_is_refused() {
    let device = Device::start(NLA);
    let mut wrong = wanted(Security::Nla, Certificate::Accepted(device.fingerprint));
    wrong.password = "not the password";

    let refused = opened(device.address, wrong).await;

    assert_eq!(refused.err(), Some(OpenError::Refused));
}
