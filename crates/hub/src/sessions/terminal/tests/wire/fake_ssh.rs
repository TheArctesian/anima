//! The device's SSH server on loopback, a `russh` server in the test (K4 Section 7): it
//! presents its host key, accepts one password (by `password` or, when told, by
//! `keyboard-interactive` only), grants or refuses the PTY and the shell, and tells the test
//! everything the hub asked of it. The test drives the shell's side through `FakeSsh`.

use std::{
    future::ready,
    net::Shutdown,
    sync::{Arc, Mutex},
};

use russh::{
    Channel, ChannelId, MethodKind, MethodSet,
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server::{self, Auth, Msg, Response, Session},
};
use tokio::{net::TcpListener, sync::mpsc};

use crate::sessions::{PublicKey, tests::fixture::PASSWORD};

/// What the hub asked of the server, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Heard {
    /// An authentication request by this method (`password`, `keyboard-interactive`, ...).
    Auth(&'static str),
    Pty {
        term: String,
        cols: u32,
        rows: u32,
        pixels: (u32, u32),
        modes: usize,
    },
    Shell,
    WindowChange {
        cols: u32,
        rows: u32,
        pixels: (u32, u32),
    },
    Input(Vec<u8>),
}

/// How the server behaves.
#[derive(Clone, Copy, Debug)]
pub(super) struct Script {
    /// The one password it accepts.
    pub(super) password: &'static str,
    /// It offers `keyboard-interactive` only, and takes the password as the prompt's answer.
    pub(super) prompts_only: bool,
    pub(super) refuse_pty: bool,
    pub(super) refuse_shell: bool,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            password: PASSWORD,
            prompts_only: false,
            refuse_pty: false,
            refuse_shell: false,
        }
    }
}

/// The server's port and host key, what the hub asked of it, and its open shell.
pub(super) struct FakeSsh {
    pub(super) port: u16,
    pub(super) key: PublicKey,
    pub(super) heard: mpsc::UnboundedReceiver<Heard>,
    shell: Arc<Mutex<Option<(server::Handle, ChannelId)>>>,
    socket: Arc<Mutex<Option<std::net::TcpStream>>>,
}

struct Device {
    script: Script,
    heard: mpsc::UnboundedSender<Heard>,
    shell: Arc<Mutex<Option<(server::Handle, ChannelId)>>>,
}

impl Device {
    fn hear(&self, heard: Heard) {
        // The test may have stopped listening; the server carries on.
        let _ = self.heard.send(heard);
    }
}

/// Every hook but the channel open answers at once, so each returns a ready future.
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
        let right = !self.script.prompts_only && user == "ops" && password == self.script.password;
        ready(Ok(verdict(right)))
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
        let Some(mut response) = response else {
            return ready(Ok(Auth::Partial {
                name: "".into(),
                instructions: "".into(),
                prompts: vec![("Password: ".into(), false)].into(),
            }));
        };
        let answer = response.next();
        let right = user == "ops" && answer.as_deref() == Some(self.script.password.as_bytes());
        ready(Ok(verdict(right)))
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
        self.hear(Heard::Pty {
            term: term.to_owned(),
            cols,
            rows,
            pixels: (pix_width, pix_height),
            modes: modes.len(),
        });
        ready(answer(session, channel, !self.script.refuse_pty))
    }

    fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.hear(Heard::Shell);
        if !self.script.refuse_shell {
            *self.shell.lock().unwrap() = Some((session.handle(), channel));
        }
        ready(answer(session, channel, !self.script.refuse_shell))
    }

    fn window_change_request(
        &mut self,
        _channel: ChannelId,
        cols: u32,
        rows: u32,
        pix_width: u32,
        pix_height: u32,
        _session: &mut Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
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
        ready(Ok(()))
    }
}

fn verdict(right: bool) -> Auth {
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

/// Starts a server presenting the ed25519 key made from `seed`, behaving as `script`.
pub(super) async fn fake_ssh(seed: u8, script: Script) -> FakeSsh {
    let host_key = PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]));
    let key = PublicKey::from_blob(host_key.public_key().to_bytes().unwrap()).unwrap();
    let methods: &[MethodKind] = if script.prompts_only {
        &[MethodKind::KeyboardInteractive]
    } else {
        &[MethodKind::Password, MethodKind::PublicKey]
    };
    let config = Arc::new(server::Config {
        keys: vec![host_key],
        methods: MethodSet::from(methods),
        auth_rejection_time: std::time::Duration::ZERO,
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (heard_tx, heard) = mpsc::unbounded_channel();
    let shell = Arc::new(Mutex::new(None));
    let socket = Arc::new(Mutex::new(None));
    let (served_shell, served_socket) = (Arc::clone(&shell), Arc::clone(&socket));
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let twin = stream.into_std().unwrap();
            *served_socket.lock().unwrap() = Some(twin.try_clone().unwrap());
            let stream = tokio::net::TcpStream::from_std(twin).unwrap();
            let device = Device {
                script,
                heard: heard_tx.clone(),
                shell: Arc::clone(&served_shell),
            };
            let config = Arc::clone(&config);
            tokio::spawn(async move {
                if let Ok(session) = server::run_stream(config, stream, device).await {
                    let _ended = session.await;
                }
            });
        }
    });
    FakeSsh {
        port,
        key,
        heard,
        shell,
        socket,
    }
}

impl FakeSsh {
    fn shell(&self) -> (server::Handle, ChannelId) {
        self.shell
            .lock()
            .unwrap()
            .clone()
            .expect("the shell is open")
    }

    /// The shell writes `bytes` to its standard output.
    pub(super) async fn say(&self, bytes: &[u8]) {
        self.try_say(bytes)
            .await
            .expect("the shell's output is read");
    }

    /// The shell writes `bytes` to its standard output; `Err` once the connection is gone.
    pub(super) async fn try_say(&self, bytes: &[u8]) -> Result<(), ()> {
        let (handle, channel) = self.shell();
        handle.data(channel, bytes.to_vec()).await.map_err(|_| ())
    }

    /// The shell writes `bytes` to its standard error.
    pub(super) async fn complain(&self, bytes: &[u8]) {
        let (handle, channel) = self.shell();
        handle
            .extended_data(channel, 1, bytes.to_vec())
            .await
            .unwrap();
    }

    /// The shell exits with status 0, as `exit` does: exit status, EOF, channel close.
    pub(super) async fn exit(&self) {
        let (handle, channel) = self.shell();
        handle.exit_status_request(channel, 0).await.unwrap();
        handle.eof(channel).await.unwrap();
        handle.close(channel).await.unwrap();
    }

    /// The device's TCP connection goes away with no SSH disconnect, as a lost network does.
    pub(super) fn cut(&self) {
        let socket = self.socket.lock().unwrap().take().expect("a connection");
        socket.shutdown(Shutdown::Both).unwrap();
    }

    /// The next thing the hub asked of the server.
    pub(super) async fn next(&mut self) -> Heard {
        tokio::time::timeout(super::WAIT, self.heard.recv())
            .await
            .expect("the server hears from the hub")
            .expect("the server is running")
    }

    /// The next thing the hub asked that is not an authentication request.
    pub(super) async fn next_after_sign_in(&mut self) -> Heard {
        loop {
            match self.next().await {
                Heard::Auth(_) => {}
                other => return other,
            }
        }
    }

    /// What the server heard so far, without waiting.
    pub(super) fn heard_so_far(&mut self) -> Vec<Heard> {
        std::iter::from_fn(|| self.heard.try_recv().ok()).collect()
    }
}
