//! The fake server's side of one connection: what it answers each hub request with, and
//! what it records. A request the hub must never send (`exec`, `env`, X11, agent, port
//! forwarding, ...) is recorded and refused, so a test that reads what was heard fails.

use std::{
    future::ready,
    sync::{Arc, Mutex},
};

use russh::{
    Channel, ChannelId,
    server::{self, Auth, Msg, Response, Session},
};
use tokio::sync::{mpsc, watch};

use super::{Heard, Hold, Ledger, Script, USER};

pub(super) struct Device {
    pub(super) script: Script,
    pub(super) heard: mpsc::UnboundedSender<Heard>,
    pub(super) shell: Arc<Mutex<Option<(server::Handle, ChannelId)>>>,
    pub(super) ledger: Arc<Mutex<Ledger>>,
    /// Opens when the test releases what the script holds.
    pub(super) gate: watch::Receiver<bool>,
    /// The keyboard-interactive rounds answered so far.
    pub(super) answered: usize,
}

impl Device {
    fn hear(&self, heard: Heard) {
        // The test may have stopped listening; the server carries on.
        let _ = self.heard.send(heard);
    }

    /// Records a request on `channel`.
    fn requested_on(&self, channel: ChannelId) {
        self.ledger.lock().unwrap().requested_on.push(channel);
    }

    /// Waits, when the script holds `what`, until the test releases it.
    fn held(&self, what: Hold) -> impl Future<Output = ()> + Send + 'static {
        let mut gate = (self.script.hold == what).then(|| self.gate.clone());
        async move {
            if let Some(gate) = gate.as_mut() {
                let _ = gate.wait_for(|open| *open).await;
            }
        }
    }
}

pub(super) fn verdict(right: bool) -> Auth {
    if right { Auth::Accept } else { Auth::reject() }
}

/// Grants or refuses a channel request.
fn answer(session: &mut Session, channel: ChannelId, granted: bool) -> Result<(), russh::Error> {
    if granted {
        session.channel_success(channel)
    } else {
        session.channel_failure(channel)
    }
}

/// Every hook answers at once unless the script holds it, so most return a ready future.
impl server::Handler for Device {
    type Error = russh::Error;

    fn auth_none(&mut self, _user: &str) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.hear(Heard::Auth("none"));
        ready(Ok(Auth::reject()))
    }

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.hear(Heard::Auth("password"));
        let right = !self.script.prompts_only && user == USER && password == self.script.password;
        let held = self.held(Hold::SignIn);
        async move {
            held.await;
            Ok(verdict(right))
        }
    }

    fn auth_publickey(
        &mut self,
        _user: &str,
        _key: server::VerifiedPublicKey<'_>,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.hear(Heard::Auth("publickey"));
        ready(Ok(Auth::reject()))
    }

    fn auth_keyboard_interactive<'a>(
        &'a mut self,
        user: &str,
        _submethods: &str,
        response: Option<Response<'a>>,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.hear(Heard::Auth("keyboard-interactive"));
        ready(Ok(match response {
            None => self.prompts(),
            Some(response) => self.answered_round(user, response),
        }))
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let opened = ("session", channel.id());
        self.ledger.lock().unwrap().opened.push(opened);
        reply.accept().await;
        Ok(())
    }

    fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        pix_width: u32,
        pix_height: u32,
        modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.requested_on(channel);
        self.hear(Heard::Pty {
            term: term.to_owned(),
            cols,
            rows,
            pixels: (pix_width, pix_height),
            modes: modes.len(),
        });
        let held = self.held(Hold::Pty);
        let granted = !self.script.refuse_pty;
        let handle = session.handle();
        async move {
            held.await;
            if granted {
                let _ = handle.channel_success(channel).await;
            } else {
                let _ = handle.channel_failure(channel).await;
            }
            Ok(())
        }
    }

    fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.requested_on(channel);
        self.hear(Heard::Shell);
        if !self.script.refuse_shell {
            *self.shell.lock().unwrap() = Some((session.handle(), channel));
        }
        ready(answer(session, channel, !self.script.refuse_shell))
    }

    fn window_change_request(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        pix_width: u32,
        pix_height: u32,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.requested_on(channel);
        self.hear(Heard::WindowChange {
            cols,
            rows,
            pixels: (pix_width, pix_height),
        });
        ready(Ok(()))
    }

    fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Input(data.to_vec()));
        let held = self.held(Hold::Input);
        async move {
            held.await;
            Ok(())
        }
    }

    fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.ledger.lock().unwrap().closed.push(channel);
        ready(Ok(()))
    }

    fn exec_request(
        &mut self,
        channel: ChannelId,
        _data: &[u8],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("exec"));
        ready(answer(session, channel, false))
    }

    fn env_request(
        &mut self,
        channel: ChannelId,
        _variable_name: &str,
        _variable_value: &str,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("env"));
        ready(answer(session, channel, false))
    }

    fn subsystem_request(
        &mut self,
        channel: ChannelId,
        _name: &str,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("subsystem"));
        ready(answer(session, channel, false))
    }

    fn x11_request(
        &mut self,
        channel: ChannelId,
        _single_connection: bool,
        _x11_auth_protocol: &str,
        _x11_auth_cookie: &str,
        _x11_screen_number: u32,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("x11-req"));
        ready(answer(session, channel, false))
    }

    fn agent_request(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        self.hear(Heard::Forbidden("auth-agent-req"));
        ready(Ok(false))
    }

    fn tcpip_forward(
        &mut self,
        _address: &str,
        _port: &mut u32,
        _session: &mut Session,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        self.hear(Heard::Forbidden("tcpip-forward"));
        ready(Ok(false))
    }

    fn streamlocal_forward(
        &mut self,
        _socket_path: &str,
        _session: &mut Session,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        self.hear(Heard::Forbidden("streamlocal-forward"));
        ready(Ok(false))
    }

    fn channel_open_direct_tcpip(
        &mut self,
        _channel: Channel<Msg>,
        _host_to_connect: &str,
        _port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        _reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("direct-tcpip"));
        ready(Ok(()))
    }

    fn channel_open_direct_streamlocal(
        &mut self,
        _channel: Channel<Msg>,
        _socket_path: &str,
        _reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("direct-streamlocal"));
        ready(Ok(()))
    }

    fn channel_open_x11(
        &mut self,
        _channel: Channel<Msg>,
        _originator_address: &str,
        _originator_port: u32,
        _reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Forbidden("x11"));
        ready(Ok(()))
    }
}
