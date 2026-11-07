//! The device's SSH server on loopback, a `russh` server in the test: it presents its host
//! key, accepts one password, answers every `exec` as told, and records each TCP connection
//! and everything the hub asked of it, so a test can show what the link did and did not send.

use std::{
    future::ready,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use russh::{
    Channel, ChannelId, MethodKind, MethodSet,
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server::{self, Auth, Msg, Session},
};
use tokio::net::TcpListener;

use crate::sessions::{PublicKey, tests::fixture::PASSWORD};

/// The user `docker-host`'s SSH settings name in the devices fixture.
const USER: &str = "operator";

/// What the hub asked of the server, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) enum Heard {
    /// An authentication request by this method.
    Auth(&'static str),
    /// A session channel opened.
    Session,
    /// An `exec` request with this command line.
    Exec(String),
    /// A channel closed by the hub.
    Close,
    /// A request or channel the link must never send, by its SSH name.
    Forbidden(&'static str),
}

/// How the server answers every `exec`.
#[derive(Clone, Debug)]
pub(in crate::sessions) enum Answer {
    /// Writes `stdout` and `stderr`, then exits with `status`.
    Exit {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        status: u32,
    },
    /// Writes the command line and a newline back, then exits 0.
    Echo,
    /// Grants the `exec` and then sends nothing.
    Hang,
    /// Grants the `exec`, then closes the channel with no exit status, as a command ended by
    /// a signal does.
    NoStatus,
    /// Refuses the `exec`.
    Refuse,
}

/// The server's port and host key, the TCP connections it accepted and what it heard.
pub(in crate::sessions) struct Server {
    pub(in crate::sessions) port: u16,
    connections: Arc<AtomicUsize>,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl Server {
    /// The TCP connections accepted so far.
    pub(in crate::sessions) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Everything heard so far.
    pub(in crate::sessions) fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }

    /// Everything heard so far except channel closes, which may arrive after a command
    /// returns.
    pub(in crate::sessions) fn requests(&self) -> Vec<Heard> {
        let mut heard = self.heard();
        heard.retain(|heard| *heard != Heard::Close);
        heard
    }
}

/// The ed25519 host key made from `seed`, as the hub stores it.
pub(in crate::sessions) fn host_key(seed: u8) -> PublicKey {
    let key = private(seed);
    PublicKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap()
}

fn private(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

/// Starts a server presenting the key made from `seed`, answering every `exec` with `answer`.
pub(in crate::sessions) async fn serve(seed: u8, answer: Answer) -> Server {
    let config = Arc::new(server::Config {
        keys: vec![private(seed)],
        methods: MethodSet::from(&[MethodKind::Password, MethodKind::PublicKey][..]),
        auth_rejection_time: std::time::Duration::ZERO,
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let connections = Arc::new(AtomicUsize::new(0));
    let heard = Arc::new(Mutex::new(Vec::new()));
    let (counted, recorded) = (Arc::clone(&connections), Arc::clone(&heard));
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            let device = Device {
                answer: answer.clone(),
                heard: Arc::clone(&recorded),
            };
            let config = Arc::clone(&config);
            tokio::spawn(async move {
                if let Ok(session) = server::run_stream(config, stream, device).await {
                    let _ended = session.await;
                }
            });
        }
    });
    Server {
        port,
        connections,
        heard,
    }
}

/// The server's side of one connection.
struct Device {
    answer: Answer,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl Device {
    fn hear(&self, heard: Heard) {
        self.heard.lock().unwrap().push(heard);
    }

    /// Records a forbidden channel request and refuses it.
    fn forbidden(
        &self,
        name: &'static str,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), russh::Error> {
        self.hear(Heard::Forbidden(name));
        session.channel_failure(channel)
    }
}

/// Sends `answer`'s output, status, EOF and close on `channel`; stops when the hub has closed
/// the channel or the connection.
async fn reply(handle: server::Handle, channel: ChannelId, answer: Answer, line: Vec<u8>) {
    let (stdout, stderr, status) = match answer {
        Answer::Exit {
            stdout,
            stderr,
            status,
        } => (stdout, stderr, status),
        Answer::Echo => ([line, b"\n".to_vec()].concat(), Vec::new(), 0),
        Answer::NoStatus => {
            let _ = async {
                handle.eof(channel).await?;
                handle.close(channel).await
            }
            .await;
            return;
        }
        Answer::Hang | Answer::Refuse => return,
    };
    let sent = async {
        handle
            .extended_data(channel, 1, stderr)
            .await
            .map_err(drop)?;
        handle.data(channel, stdout).await.map_err(drop)?;
        handle.exit_status_request(channel, status).await?;
        handle.eof(channel).await?;
        handle.close(channel).await
    };
    // The hub may have closed the channel first, which ends the reply.
    let _ = sent.await;
}

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
        let right = user == USER && password == PASSWORD;
        ready(Ok(if right { Auth::Accept } else { Auth::reject() }))
    }

    fn auth_publickey(
        &mut self,
        _user: &str,
        _key: server::VerifiedPublicKey<'_>,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.hear(Heard::Auth("publickey"));
        ready(Ok(Auth::reject()))
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.hear(Heard::Session);
        reply.accept().await;
        Ok(())
    }

    fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Exec(String::from_utf8_lossy(data).into_owned()));
        let answered = if matches!(self.answer, Answer::Refuse) {
            session.channel_failure(channel)
        } else {
            let sending = reply(
                session.handle(),
                channel,
                self.answer.clone(),
                data.to_vec(),
            );
            tokio::spawn(sending);
            session.channel_success(channel)
        };
        ready(answered)
    }

    fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Close);
        ready(Ok(()))
    }

    fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _cols: u32,
        _rows: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(self.forbidden("pty-req", channel, session))
    }

    fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(self.forbidden("shell", channel, session))
    }

    fn env_request(
        &mut self,
        channel: ChannelId,
        _variable_name: &str,
        _variable_value: &str,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(self.forbidden("env", channel, session))
    }

    fn subsystem_request(
        &mut self,
        channel: ChannelId,
        _name: &str,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(self.forbidden("subsystem", channel, session))
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
        ready(self.forbidden("x11-req", channel, session))
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
}
