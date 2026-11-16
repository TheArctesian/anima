//! One connection to the test device: the acceptor's opening, then the active session,
//! where the device notes the client's input and does what the test tells it.

use std::rc::Rc;
use std::sync::Arc;

use ironrdp_acceptor::{Acceptor, AcceptorResult, accept_credssp, accept_finalize};
use ironrdp_connector::sspi::generator::NetworkRequest;
use ironrdp_connector::{ConnectorResult, DesktopSize, ServerName, general_err};
use ironrdp_core::WriteBuf;
use ironrdp_pdu::Action;
use ironrdp_pdu::nego::SecurityProtocol;
use ironrdp_pdu::rdp::client_info::Credentials;
use ironrdp_svc::StaticChannelSet;
use ironrdp_tokio::{FramedWrite as _, NetworkClient, TokioFramed, single_sequence_step_read};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::input::heard;
use super::screen::{capabilities, deactivate_all, fill};
use super::{Offered, Seen, Served, Tell};

type Framed = TokioFramed<TlsStream<TcpStream>>;

/// Why a connection ended. The device does not say why: the test sees `Seen::Ended`, and
/// the client's side of the test sees its own error.
struct Ended;

impl<E: std::error::Error> From<E> for Ended {
    fn from(_: E) -> Self {
        Self
    }
}

/// NTLM asks no other host.
struct NoNetwork;

impl NetworkClient for NoNetwork {
    fn send(&mut self, _: &NetworkRequest) -> impl Future<Output = ConnectorResult<Vec<u8>>> {
        std::future::ready(Err(general_err!("the test device asks no other host")))
    }
}

/// Serves `client` to its end, then notes the end.
pub(super) async fn serve(client: TcpStream, served: Rc<Served>) {
    // Every way a connection ends is the same to the test: the client's side says why.
    let _ended: Result<(), Ended> = session(client, &served).await;
    // The test may have stopped listening; the device has nobody else to tell.
    let _told = served.seen.send(Seen::Ended).await;
}

async fn session(client: TcpStream, served: &Served) -> Result<(), Ended> {
    let (width, height) = served.setup.size;
    let (security, account) = match served.setup.offered {
        Offered::Tls => (SecurityProtocol::SSL, None),
        Offered::Nla { username, password } => (
            SecurityProtocol::HYBRID | SecurityProtocol::HYBRID_EX,
            Some(Credentials {
                username: username.to_owned(),
                password: password.to_owned(),
                domain: None,
            }),
        ),
    };
    let size = DesktopSize { width, height };
    let mut acceptor = Acceptor::new(security, size, capabilities(size), account);
    let plain = negotiated(client, &mut acceptor).await?;
    let stream = TlsAcceptor::from(Arc::clone(&served.tls))
        .accept(plain)
        .await?;
    acceptor.mark_security_upgrade_as_done();
    let mut framed = Framed::new(stream);
    let name = ServerName::new("anima-test-device");
    let key = served.public_key.clone();
    accept_credssp(&mut framed, &mut acceptor, &mut NoNetwork, name, key, None).await?;
    let (framed, result) = accept_finalize(framed, &mut acceptor).await?;
    if let Some(account) = &result.credentials {
        let sign_in = Seen::SignIn {
            username: account.username.clone(),
            password: account.password.clone(),
            domain: account.domain.clone(),
        };
        served.seen.send(sign_in).await?;
    }
    active(framed, acceptor, result, served).await
}

/// The X.224 negotiation, up to TLS. A refusal is sent before the acceptor's error ends
/// the connection, so the client learns why.
async fn negotiated(client: TcpStream, acceptor: &mut Acceptor) -> Result<TcpStream, Ended> {
    let mut framed = TokioFramed::new(client);
    let mut buf = WriteBuf::new();
    while acceptor.reached_security_upgrade().is_none() {
        buf.clear();
        let step = single_sequence_step_read(&mut framed, acceptor, &mut buf).await;
        if buf.filled_len() > 0 {
            framed.write_all(buf.filled()).await?;
        }
        step?;
    }
    Ok(framed.into_inner_no_leftover())
}

/// The active session, until the client goes or the test closes it.
async fn active(
    mut framed: Framed,
    mut acceptor: Acceptor,
    mut result: AcceptorResult,
    served: &Served,
) -> Result<(), Ended> {
    let (tell, mut told) = mpsc::channel(8);
    served.active.send_replace(Some(tell.clone()));
    let ended = loop {
        for frame in std::mem::take(&mut result.input_events) {
            note(&frame, served).await?;
        }
        let tell = tokio::select! {
            pdu = framed.read_pdu() => match pdu {
                // Slow-path PDUs and channel data are of no use to the device.
                Ok((Action::FastPath, frame)) => { note(&frame, served).await?; continue },
                Ok((Action::X224, _)) => continue,
                Err(error) => break Err(error.into()),
            },
            tell = told.recv() => tell,
        };
        match tell {
            Some(Tell::Paint { area, colour }) => framed.write_all(&fill(area, colour)).await?,
            Some(Tell::Resize { width, height }) => {
                framed.write_all(&deactivate_all(&result)).await?;
                let size = DesktopSize { width, height };
                acceptor = Acceptor::new_deactivation_reactivation(
                    acceptor,
                    StaticChannelSet::new(),
                    size,
                )?;
                (framed, result) = accept_finalize(framed, &mut acceptor).await?;
            }
            Some(Tell::Close) | None => break Ok(()),
        }
    };
    served.active.send_if_modified(|now| {
        let ours = now
            .as_ref()
            .is_some_and(|active| active.same_channel(&tell));
        if ours {
            *now = None;
        }
        ours
    });
    ended
}

/// Notes the input of one fast-path frame.
async fn note(frame: &[u8], served: &Served) -> Result<(), Ended> {
    for seen in heard(frame) {
        served.seen.send(seen).await?;
    }
    Ok(())
}
