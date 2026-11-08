//! A device's SSH server on loopback, a `russh` server in the test: it presents its host key,
//! accepts one password, answers each `exec` line from a table the test can change (or never
//! answers it), and records the connections, the lines and any request a collection must
//! never send.

use std::{
    collections::HashMap,
    future::ready,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use russh::{
    Channel, ChannelId, MethodKind, MethodSet,
    keys::{PrivateKey, PublicKey, ssh_key::private::Ed25519Keypair},
    server::{self, Auth, Msg, Session},
};
use tokio::net::TcpListener;

/// The user the devices fixture names, and the password the test stores for it.
const USER: &str = "operator";
pub(super) const PASSWORD: &str = "metrics test password";
/// The seed of the server's ed25519 host key.
const SEED: u8 = 11;

/// What each command line prints; a line not in it exits 127 with no output.
pub(super) type Answers = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// How the server misbehaves on a line instead of answering it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Accepts the line and never prints, exits or closes, as a hung tool.
    Stall,
    /// Ends the whole connection, as a device that goes away.
    HangUp,
}

/// The lines the server misbehaves on.
type Faults = Arc<Mutex<HashMap<String, Fault>>>;

/// The server's port, the table it answers from, and what it saw.
pub(super) struct Server {
    pub(super) port: u16,
    pub(super) answers: Answers,
    faults: Faults,
    connections: Arc<AtomicUsize>,
    heard: Arc<Mutex<Vec<String>>>,
}

impl Server {
    /// The TCP connections accepted so far.
    pub(super) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Each `exec` line in order, and `!name` for each forbidden request.
    pub(super) fn heard(&self) -> Vec<String> {
        self.heard.lock().unwrap().clone()
    }

    /// Sets what `line` prints.
    pub(super) fn answer(&self, line: &str, stdout: &[u8]) {
        self.answers
            .lock()
            .unwrap()
            .insert(line.to_owned(), stdout.to_vec());
    }

    /// Makes `line` run without ever printing, exiting or closing, as a hung tool.
    pub(super) fn stall(&self, line: &str) {
        self.faults
            .lock()
            .unwrap()
            .insert(line.to_owned(), Fault::Stall);
    }

    /// Makes `line` end the connection, as a device that goes away.
    pub(super) fn hang_up_on(&self, line: &str) {
        self.faults
            .lock()
            .unwrap()
            .insert(line.to_owned(), Fault::HangUp);
    }

    /// Makes `line` answer from the table again.
    pub(super) fn unstall(&self, line: &str) {
        self.faults.lock().unwrap().remove(line);
    }
}

/// The host key the server presents. `metrics` may not import `trust`, so the test reads its
/// stored form with `ssh-key`, not with the hub's own key type.
pub(super) fn host_key() -> PublicKey {
    private().public_key().clone()
}

fn private() -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[SEED; 32]))
}

pub(super) async fn serve() -> Server {
    let config = Arc::new(server::Config {
        keys: vec![private()],
        methods: MethodSet::from(&[MethodKind::Password][..]),
        auth_rejection_time: std::time::Duration::ZERO,
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let answers = Answers::default();
    let faults = Faults::default();
    let connections = Arc::new(AtomicUsize::new(0));
    let heard = Arc::new(Mutex::new(Vec::new()));
    let device = Device {
        answers: Arc::clone(&answers),
        faults: Arc::clone(&faults),
        heard: Arc::clone(&heard),
    };
    let counted = Arc::clone(&connections);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            let (config, device) = (Arc::clone(&config), device.clone());
            tokio::spawn(async move {
                if let Ok(session) = server::run_stream(config, stream, device).await {
                    let _ended = session.await;
                }
            });
        }
    });
    Server {
        port,
        answers,
        faults,
        connections,
        heard,
    }
}

/// The server's side of one connection.
#[derive(Clone)]
struct Device {
    answers: Answers,
    faults: Faults,
    heard: Arc<Mutex<Vec<String>>>,
}

impl Device {
    fn hear(&self, heard: String) {
        self.heard.lock().unwrap().push(heard);
    }

    fn forbidden(
        &self,
        name: &str,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), russh::Error> {
        self.hear(format!("!{name}"));
        session.channel_failure(channel)
    }
}

/// Sends `stdout`, the exit status, EOF and close on `channel`.
async fn reply(handle: server::Handle, channel: ChannelId, stdout: Option<Vec<u8>>) {
    let status = if stdout.is_some() { 0 } else { 127 };
    let sent = async {
        handle
            .data(channel, stdout.unwrap_or_default())
            .await
            .map_err(drop)?;
        handle.exit_status_request(channel, status).await?;
        handle.eof(channel).await?;
        handle.close(channel).await
    };
    // The hub may have closed the channel first, which ends the reply.
    let _ = sent.await;
}

impl server::Handler for Device {
    type Error = russh::Error;

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        let right = user == USER && password == PASSWORD;
        ready(Ok(if right { Auth::Accept } else { Auth::reject() }))
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let line = String::from_utf8_lossy(data).into_owned();
        let stdout = self.answers.lock().unwrap().get(&line).cloned();
        let fault = self.faults.lock().unwrap().get(&line).copied();
        self.hear(line);
        match fault {
            Some(Fault::HangUp) => return ready(Err(russh::Error::Disconnect)),
            Some(Fault::Stall) => {}
            None => {
                tokio::spawn(reply(session.handle(), channel, stdout));
            }
        }
        ready(session.channel_success(channel))
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
}
