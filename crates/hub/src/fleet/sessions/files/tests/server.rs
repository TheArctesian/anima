//! The device's SSH server on loopback, a `russh` server in the test: it presents its host
//! key, accepts one password, serves the `sftp` subsystem over a temporary directory with
//! `russh-sftp`'s server half, and records everything the hub asked of it, so a test can show
//! the hub asked for the `sftp` subsystem and nothing else.

use std::{
    future::ready,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use russh::{
    Channel, ChannelId, MethodKind, MethodSet,
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server::{self, Auth, Msg, Session},
};
use tempfile::TempDir;
use tokio::net::TcpListener;

use super::sftp::Disk;
use crate::fleet::{sessions::tests::fixture::PASSWORD, trust::PublicKey};

/// The user `docker-host`'s SSH settings name in the devices fixture.
const USER: &str = "operator";

/// What the hub asked of the server, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Heard {
    /// A sign-in by this method.
    Auth(&'static str),
    /// A session channel opened.
    Session,
    /// A subsystem request by name.
    Subsystem(String),
    /// An SFTP open of this path with these SFTP flags.
    Open { path: String, flags: u32 },
    /// A request or channel the files tunnel must never send, by its SSH name.
    Forbidden(&'static str),
}

/// How the server misbehaves, switched by a test while it runs.
#[derive(Default)]
pub(super) struct Quirks {
    /// `lstat` says nothing is there, whatever is there.
    pub(super) hide_files: AtomicBool,
    /// Reads are never answered.
    pub(super) stall_reads: AtomicBool,
    /// Each read waits 100 ms before it is answered.
    pub(super) slow_reads: AtomicBool,
    /// Each open makes its file at once and answers 500 ms later.
    pub(super) slow_opens: AtomicBool,
}

/// The server's port and host key, its directory and what it heard.
pub(super) struct Server {
    pub(super) port: u16,
    pub(super) key: PublicKey,
    pub(super) root: TempDir,
    pub(super) quirks: Arc<Quirks>,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl Server {
    /// Everything heard so far.
    pub(super) fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }

    /// Everything heard but sign-ins and SFTP opens: the SSH requests and channels.
    pub(super) fn requests(&self) -> Vec<Heard> {
        let mut heard = self.heard();
        heard.retain(|heard| !matches!(heard, Heard::Auth(_) | Heard::Open { .. }));
        heard
    }

    /// The SFTP opens heard so far.
    pub(super) fn opens(&self) -> Vec<(String, u32)> {
        self.heard()
            .into_iter()
            .filter_map(|heard| match heard {
                Heard::Open { path, flags } => Some((path, flags)),
                _ => None,
            })
            .collect()
    }

    /// Writes `bytes` to `name` in the served directory.
    pub(super) fn put(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.root.path().join(name), bytes).unwrap();
    }

    /// The bytes of `name` in the served directory, if it is there.
    pub(super) fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.path().join(name)).ok()
    }
}

fn private(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

/// Starts a server presenting the key made from `seed`, serving a new empty directory.
pub(super) async fn serve(seed: u8) -> Server {
    let key = private(seed);
    let public = PublicKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap();
    let config = Arc::new(server::Config {
        keys: vec![key],
        methods: MethodSet::from(&[MethodKind::Password][..]),
        auth_rejection_time: std::time::Duration::ZERO,
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let root = tempfile::tempdir().unwrap();
    let (heard, quirks) = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Quirks::default()),
    );
    let device = Device {
        disk: Disk::new(root.path(), Arc::clone(&heard), Arc::clone(&quirks)),
        heard: Arc::clone(&heard),
        channels: Vec::new(),
    };
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let device = device.fresh();
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
        key: public,
        root,
        quirks,
        heard,
    }
}

/// The server's side of one connection.
struct Device {
    disk: Disk,
    heard: Arc<Mutex<Vec<Heard>>>,
    /// Session channels not yet given a subsystem.
    channels: Vec<Channel<Msg>>,
}

impl Device {
    fn fresh(&self) -> Self {
        Self {
            disk: self.disk.clone(),
            heard: Arc::clone(&self.heard),
            channels: Vec::new(),
        }
    }

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

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.hear(Heard::Session);
        self.channels.push(channel);
        reply.accept().await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.hear(Heard::Subsystem(name.to_owned()));
        let at = self.channels.iter().position(|channel| channel.id() == id);
        match at {
            Some(at) if name == "sftp" => {
                let channel = self.channels.swap_remove(at);
                session.channel_success(id)?;
                russh_sftp::server::run(channel.into_stream(), self.disk.clone()).await;
                Ok(())
            }
            _ => session.channel_failure(id),
        }
    }

    fn exec_request(
        &mut self,
        channel: ChannelId,
        _data: &[u8],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(self.forbidden("exec", channel, session))
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
}

/// Whether `quirk` is switched on.
pub(super) fn on(quirk: &AtomicBool) -> bool {
    quirk.load(Ordering::SeqCst)
}
