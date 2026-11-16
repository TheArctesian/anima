//! The opening of an RDP session, on `IronRDP`: the X.224 negotiation, TLS, the check of
//! the device's certificate, the sign-in inside TLS when the device asks for it (NLA), and
//! the exchange of capabilities up to the active session.
//!
//! No credential leaves before the certificate is checked (SEC-007). No error holds a
//! value from the device or the account (LOG-007).
//!
//! `IronRDP` keeps the account for the whole session, to exchange capabilities again after
//! a resize, and does not zero it. The worker process holds one session and ends with it.
//!
//! I/O edge.

use std::sync::Arc;

use ironrdp_cliprdr::CliprdrClient;
use ironrdp_connector::sspi::generator::NetworkRequest;
use ironrdp_connector::{
    ClientConnector, Config, ConnectionOutcome, ConnectionResult, ConnectorErrorKind,
    ConnectorResult, Credentials, DesktopSize, ServerName, general_err,
};
use ironrdp_displaycontrol::client::DisplayControlClient;
use ironrdp_dvc::DrdynvcClient;
use ironrdp_pdu::gcc::{ConnectionType, KeyboardType};
use ironrdp_pdu::rdp::capability_sets::{MajorPlatformType, RailSupportLevel};
use ironrdp_pdu::rdp::client_info::{PerformanceFlags, TimezoneInfo};
use ironrdp_tokio::{NetworkClient, TokioFramed};
use rustls::pki_types::ServerName as TlsName;
use sha2::{Digest as _, Sha256};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use super::certificate::any_certificate;
use super::clipboard::Clipboard;
use crate::handshake::{Certificate, Security};

/// The US keyboard, the layout of the key table (`keys`).
const US_KEYBOARD: u32 = 0x409;
/// The name the device shows for the client's computer.
const CLIENT_NAME: &str = "anima";

/// The connection to a signed-in device.
pub type Connection = TokioFramed<TlsStream<TcpStream>>;

/// Why an RDP session did not open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("the device did not answer as an RDP server")]
    NotRdp,
    #[error("the device does not offer the security the session must use")]
    Security,
    #[error("the device's certificate is not the accepted one")]
    Certificate,
    #[error("the device refused the sign-in")]
    Refused,
    #[error("the opening did not finish")]
    Failed,
}

/// What the session asks of the device.
pub struct Wanted<'a> {
    /// Empty when there is none: the device then shows its own sign-in.
    pub username: &'a str,
    pub password: &'a str,
    pub domain: Option<&'a str>,
    pub security: Security,
    pub certificate: Certificate,
    /// The size of the screen to ask for, in pixels.
    pub size: (u16, u16),
}

/// A session with its device signed in.
pub struct Opened {
    pub connection: Connection,
    pub result: ConnectionResult,
}

/// Kerberos would ask a domain controller. The gateway signs in with NTLM, which asks
/// nobody.
struct NoNetwork;

impl NetworkClient for NoNetwork {
    fn send(&mut self, _: &NetworkRequest) -> impl Future<Output = ConnectorResult<Vec<u8>>> {
        let asks_nobody = general_err!("the gateway asks no other host about a sign-in");
        std::future::ready(Err(asks_nobody))
    }
}

/// Opens an RDP session on `device`, a new connection to it.
///
/// # Errors
/// [`OpenError`]: each way the opening can end. The caller sets the time limit.
pub async fn open(
    device: TcpStream,
    wanted: Wanted<'_>,
    clipboard: Clipboard,
) -> Result<Opened, OpenError> {
    let (Ok(local), Ok(peer)) = (device.local_addr(), device.peer_addr()) else {
        return Err(OpenError::Failed);
    };
    let display =
        DrdynvcClient::new().with_dynamic_channel(DisplayControlClient::new(|_| Ok(Vec::new())));
    let mut connector = ClientConnector::new(config(&wanted), local)
        .with_static_channel(display)
        .with_static_channel(CliprdrClient::new(Box::new(clipboard)));

    let mut plain = TokioFramed::new(device);
    let upgrade = ironrdp_tokio::connect_begin(&mut plain, &mut connector)
        .await
        .map_err(|error| match error.kind() {
            // The device refused what was asked, or picked what was not asked.
            ConnectorErrorKind::Negotiation(_) | ConnectorErrorKind::Reason(_) => {
                OpenError::Security
            }
            _ => OpenError::NotRdp,
        })?;

    let tls = TlsConnector::from(Arc::new(any_certificate().map_err(|_| OpenError::Failed)?));
    let name = TlsName::IpAddress(peer.ip().into());
    let stream = tls
        .connect(name, plain.into_inner_no_leftover())
        .await
        .map_err(|_| OpenError::Failed)?;
    // The handshake is done and nothing of the account has been sent.
    let shown = stream
        .get_ref()
        .1
        .peer_certificates()
        .and_then(<[_]>::first);
    let shown = shown.ok_or(OpenError::Certificate)?;
    if let Certificate::Accepted(accepted) = wanted.certificate
        && Sha256::digest(shown).as_slice() != accepted
    {
        return Err(OpenError::Certificate);
    }
    let public_key = public_key(shown).ok_or(OpenError::Certificate)?;

    let upgraded = ironrdp_tokio::mark_as_upgraded(upgrade, &mut connector);
    let mut connection = TokioFramed::new(stream);
    let outcome = ironrdp_tokio::connect_finalize(
        upgraded,
        connector,
        &mut connection,
        &mut NoNetwork,
        ServerName::new(peer.ip().to_string()),
        public_key,
        None,
    )
    .await
    .map_err(|error| match error.kind() {
        ConnectorErrorKind::Credssp(_) | ConnectorErrorKind::AccessDenied => OpenError::Refused,
        _ => OpenError::Failed,
    })?;
    match outcome {
        ConnectionOutcome::Connected(result) => Ok(Opened { connection, result }),
        // A device that sends the session to another host: the hub checked this one only.
        ConnectionOutcome::Redirected(_) => Err(OpenError::Failed),
    }
}

/// The key of the certificate, which the sign-in inside TLS is bound to.
fn public_key(certificate: &[u8]) -> Option<Vec<u8>> {
    let (_, parsed) = x509_parser::parse_x509_certificate(certificate).ok()?;
    Some(parsed.public_key().subject_public_key.data.to_vec())
}

fn config(wanted: &Wanted<'_>) -> Config {
    let (width, height) = wanted.size;
    Config {
        desktop_size: DesktopSize { width, height },
        monitor_layout: None,
        desktop_scale_factor: 0,
        desktop_physical_size: None,
        desktop_orientation: None,
        device_scale_factor: None,
        monitor_extended_layout: None,
        relative_mouse_input: false,
        enable_tls: wanted.security != Security::Nla,
        enable_credssp: wanted.security.may_be_nla(),
        enable_standard_rdp_security: false,
        credentials: Credentials::UsernamePassword {
            username: wanted.username.to_owned(),
            password: wanted.password.to_owned(),
        },
        domain: wanted.domain.map(str::to_owned),
        client_build: 0,
        client_name: CLIENT_NAME.to_owned(),
        keyboard_type: KeyboardType::IBM_ENHANCED,
        keyboard_subtype: 0,
        keyboard_functional_keys_count: 12,
        keyboard_layout: US_KEYBOARD,
        connection_type: ConnectionType::Lan,
        ime_file_name: String::new(),
        bitmap: None,
        dig_product_id: String::new(),
        client_dir: String::new(),
        alternate_shell: String::new(),
        work_dir: String::new(),
        remote_application_mode: false,
        rail_support_level: RailSupportLevel::empty(),
        platform: MajorPlatformType::UNSPECIFIED,
        hardware_id: None,
        request_data: None,
        autologon: !wanted.username.is_empty(),
        enable_audio_playback: false,
        enable_audio_capture: false,
        performance_flags: PerformanceFlags::default(),
        license_cache: None,
        timezone_info: TimezoneInfo::default(),
        compression_type: None,
        // The device's pointer is drawn into the picture, as for VNC.
        enable_server_pointer: true,
        pointer_software_rendering: true,
        multitransport_flags: None,
        support_dyn_vc_gfx_protocol: false,
    }
}
