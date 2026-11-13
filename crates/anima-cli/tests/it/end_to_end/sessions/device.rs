//! The device: an SSH server on loopback, a `russh` server in the test. It presents a host key,
//! accepts one user and password, runs a tiny shell (`echo <words>` and `exit`) on a terminal
//! channel, serves the `sftp` subsystem over a temporary directory, and counts every
//! authentication attempt, so a test can show the hub sent none (SEC-011).

use std::future::ready;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use russh::keys::{PrivateKey, ssh_key::private::Ed25519Keypair};
use russh::server::{self, Auth, Msg, Session};
use russh::{Channel, ChannelId, MethodKind, MethodSet};
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};

use super::sftp::Disk;

pub const USER: &str = "operator";
pub const PASSWORD: &str = "device password for the e2e";

/// A running device: its port, its directory and how many sign-ins it was asked for.
pub struct Device {
    pub port: u16,
    pub root: TempDir,
    attempts: Arc<AtomicUsize>,
}

impl Device {
    /// Starts a device presenting the Ed25519 key made from `seed`.
    pub async fn start(seed: u8) -> Self {
        let key = PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]));
        let config = Arc::new(server::Config {
            keys: vec![key],
            methods: MethodSet::from(&[MethodKind::Password][..]),
            auth_rejection_time: std::time::Duration::ZERO,
            ..server::Config::default()
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let port = listener.local_addr().expect("an address").port();
        let root = tempfile::tempdir().expect("a directory");
        let attempts = Arc::new(AtomicUsize::new(0));
        let connection = Connection {
            disk: Disk::new(root.path()),
            attempts: Arc::clone(&attempts),
            pending: Vec::new(),
            typed: Vec::new(),
        };
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(Arc::clone(&config), stream, connection.fresh()));
            }
        });
        Self {
            port,
            root,
            attempts,
        }
    }

    /// How many authentication attempts reached the device.
    pub fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }
}

/// Serves one SSH connection until it ends.
async fn serve(config: Arc<server::Config>, stream: TcpStream, connection: Connection) {
    if let Ok(session) = server::run_stream(config, stream, connection).await {
        let _ended = session.await;
    }
}

/// The device's side of one connection.
struct Connection {
    disk: Disk,
    attempts: Arc<AtomicUsize>,
    /// Session channels not yet given a subsystem.
    pending: Vec<Channel<Msg>>,
    /// The shell's input not yet ended by a line break.
    typed: Vec<u8>,
}

impl Connection {
    fn fresh(&self) -> Self {
        Self {
            disk: self.disk.clone(),
            attempts: Arc::clone(&self.attempts),
            pending: Vec::new(),
            typed: Vec::new(),
        }
    }

    /// Runs one shell line: `echo` prints its words, `exit` ends the shell.
    fn run_line(line: &str, channel: ChannelId, session: &mut Session) -> Result<(), russh::Error> {
        let line = line.trim();
        if let Some(words) = line.strip_prefix("echo ") {
            return session.data(channel, format!("{words}\r\n").into_bytes());
        }
        if line == "exit" {
            session.exit_status_request(channel, 0)?;
            session.eof(channel)?;
            return session.close(channel);
        }
        Ok(())
    }
}

impl server::Handler for Connection {
    type Error = russh::Error;

    fn auth_none(&mut self, _user: &str) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        ready(Ok(Auth::reject()))
    }

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        let right = user == USER && password == PASSWORD;
        ready(Ok(if right { Auth::Accept } else { Auth::reject() }))
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.pending.push(channel);
        reply.accept().await;
        Ok(())
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
        ready(session.channel_success(channel))
    }

    fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(session.channel_success(channel))
    }

    fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.typed.extend_from_slice(data);
        let mut result = Ok(());
        while let Some(end) = self.typed.iter().position(|&b| b == b'\r' || b == b'\n') {
            let line: Vec<u8> = self.typed.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line).into_owned();
            result = result.and_then(|()| Self::run_line(&line, channel, session));
        }
        ready(result)
    }

    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let at = self.pending.iter().position(|channel| channel.id() == id);
        match at {
            Some(at) if name == "sftp" => {
                let channel = self.pending.swap_remove(at);
                session.channel_success(id)?;
                russh_sftp::server::run(channel.into_stream(), self.disk.clone()).await;
                Ok(())
            }
            _ => session.channel_failure(id),
        }
    }
}
