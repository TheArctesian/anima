//! The hub's side of an SSH connection's key exchange and the channels the device offers
//! (SEC-007, K4 Section 8.3): the presented key is reported once and must be the expected
//! one, and every channel the device opens towards the hub is refused.

use russh::{
    Channel, ChannelOpenFailure,
    client::{self, ChannelOpenHandle, Msg, Session},
    keys::PublicKeyOrCertificate,
};
use tokio::sync::oneshot;

use crate::fleet::trust::PublicKey;

/// The hub's side of the key exchange: reports the presented key once, and lets the
/// connection go on only when it is the expected key. A certificate, or a key the hub does
/// not accept, is reported as `None` and never matches (SEC-007, K4 Section 8.3).
///
/// The device may open no channel towards the hub: a session, X11, agent, TCP/IP or
/// stream-local forwarding channel is refused, as is one of a type russh does not know
/// (K4 Section 8.4: one session channel, opened by the hub).
pub(crate) struct Observer {
    expected: Option<PublicKey>,
    seen: Option<oneshot::Sender<Option<PublicKey>>>,
}

impl Observer {
    /// An observer that lets the connection go on only for `expected`, and the receiver of
    /// the key the host presents. The receiver fails when no key exchange reached the key.
    pub(crate) fn expecting(
        expected: Option<PublicKey>,
    ) -> (Self, oneshot::Receiver<Option<PublicKey>>) {
        let (seen, receiver) = oneshot::channel();
        let observer = Self {
            expected,
            seen: Some(seen),
        };
        (observer, receiver)
    }

    /// The key as the hub stores it, rebuilt from the bytes russh decoded: a certificate or a
    /// key the hub does not accept counts as no key.
    fn presented(presented: &PublicKeyOrCertificate) -> Option<PublicKey> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
            return None;
        };
        PublicKey::from_blob(key.to_bytes().ok()?).ok()
    }
}

/// Refuses a channel the device opened; the channel itself is dropped unused.
async fn refused(channel: Channel<Msg>, reply: ChannelOpenHandle) -> Result<(), russh::Error> {
    drop(channel);
    reply
        .reject(ChannelOpenFailure::AdministrativelyProhibited)
        .await;
    Ok(())
}

impl client::Handler for Observer {
    type Error = russh::Error;

    /// russh has verified the exchange signature with this key before it calls this, and
    /// calls it for the first key exchange only. A later exchange (a rekey) runs inside the
    /// encrypted connection, so only the device that proved the expected key can take part;
    /// russh does not check its key again.
    fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let presented = Self::presented(server_public_key);
        let go_on = presented.is_some() && presented == self.expected;
        if let Some(seen) = self.seen.take() {
            // The receiver is gone only when the caller was dropped; nobody is left to tell.
            let _gone = seen.send(presented);
        }
        std::future::ready(Ok(go_on))
    }

    fn server_channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_x11(
        &mut self,
        channel: Channel<Msg>,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_agent_forward(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _host_to_connect: &str,
        _port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_forwarded_streamlocal(
        &mut self,
        channel: Channel<Msg>,
        _socket_path: &str,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    fn server_channel_open_direct_streamlocal(
        &mut self,
        channel: Channel<Msg>,
        _socket_path: &str,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        refused(channel, reply)
    }

    /// Unknown channel types stay refused, as russh's default does; said here so it is not
    /// left to a default that could change.
    fn should_accept_unknown_server_channel(
        &mut self,
        _id: russh::ChannelId,
        _channel_type: &str,
    ) -> impl Future<Output = bool> + Send {
        std::future::ready(false)
    }
}
