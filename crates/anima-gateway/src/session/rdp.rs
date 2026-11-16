//! An RDP session from `connect` to its end: a prompt for the account when the device
//! will ask for it before the session (GW-015), the opening, and then the open session
//! (`running`). Each way the opening can fail is told to the hub with the status that
//! gives the session its end reason (K4 Section 5). I/O edge.

mod running;

use ironrdp_displaycontrol::pdu::MonitorLayoutEntry;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::hub::Hub;
use super::opening::{Allowed, ask_for_missing, refuse};
use super::{Ended, Settings, status};
use crate::handshake::{Credential, Opening, Rdp, Secret};
use crate::rdp::{Clipboard, OpenError, Wanted, open};

/// What the hub set for an open session.
struct Terms {
    allowed: Allowed,
    /// The device is asked for a screen of the size of the client's view.
    resizes: bool,
}

/// Opens the session `opening` asks for on its device and runs it to its end.
pub(super) async fn serve<S>(
    hub: &mut Hub<S>,
    mut opening: Opening,
    rdp: &Rdp,
    settings: &Settings,
) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // A device that signs in before the session (NLA) must get the whole account at
    // once. The operator is asked before the device is reached, so the device does not
    // wait with an open connection while the operator types.
    if rdp.security.may_be_nla() {
        let account = [Credential::Username, Credential::Password];
        if let Err(ended) = ask_for_missing(hub, &mut opening, &account, settings).await {
            return ended;
        }
    }
    let connecting = timeout(settings.device_wait, TcpStream::connect(opening.device)).await;
    let Ok(Ok(device)) = connecting else {
        return refuse(hub, status::UPSTREAM_NOT_FOUND, Ended::DeviceUnreachable).await;
    };
    // Keys and pointer moves are small: do not hold them back (Nagle).
    let _ = device.set_nodelay(true);
    let wanted = Wanted {
        username: text(opening.username.as_ref()),
        password: text(opening.password.as_ref()),
        domain: rdp.domain.as_deref(),
        security: rdp.security,
        certificate: rdp.certificate,
        size: asked_size(opening.view),
    };
    // The device's offer and its sign-in: each has the wait of a step.
    let opened = timeout(
        2 * settings.device_wait,
        open(device, wanted, Clipboard::default()),
    )
    .await;
    let terms = Terms {
        allowed: Allowed {
            copy: opening.copy,
            paste: opening.paste,
            formats: opening.formats,
            view: opening.view,
        },
        resizes: rdp.resize,
    };
    // The gateway's copies of the account are zeroed here.
    drop(opening);
    let (status, ended) = match opened {
        Ok(Ok(opened)) => return running::run(hub, opened, terms, settings).await,
        Ok(Err(OpenError::Refused)) => (status::CLIENT_UNAUTHORIZED, Ended::SignInRefused),
        Ok(Err(OpenError::NotRdp)) => (status::UPSTREAM_ERROR, Ended::NotRdp),
        Ok(Err(OpenError::Security)) => (status::UPSTREAM_ERROR, Ended::SecurityNotOffered),
        // The hub read the certificate first and tells the operator when it changed (K4
        // Section 6). One that changes between the two reads ends as a gateway error.
        Ok(Err(OpenError::Certificate)) => (status::UPSTREAM_ERROR, Ended::OtherCertificate),
        Ok(Err(OpenError::Failed)) | Err(_) => (status::UPSTREAM_ERROR, Ended::SignInFailed),
    };
    refuse(hub, status, ended).await
}

/// The text of a part of the account; empty when there is none.
fn text(secret: Option<&Secret>) -> &str {
    secret.map_or("", Secret::expose)
}

/// The screen size to ask the device for: the client's view, within what RDP allows.
fn asked_size((width, height): (usize, usize)) -> (u16, u16) {
    let side = |side: usize| u32::try_from(side).unwrap_or(u32::MAX);
    let (width, height) = MonitorLayoutEntry::adjust_display_size(side(width), side(height));
    let side = |side: u32| u16::try_from(side).unwrap_or(u16::MAX);
    (side(width), side(height))
}
