//! An in-process SSH server on the loopback interface for the connection test: it presents
//! the host keys it is given and records every authentication attempt it receives.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use russh::{
    keys::{
        Algorithm, EcdsaCurve, PrivateKey, PublicKey,
        ssh_key::{LineEnding, private::Ed25519Keypair},
    },
    server::{self, Auth, PublicKeyOffer, VerifiedPublicKey},
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// The user name and secrets the server accepts.
pub const USER: &str = "operator";
pub const PASSWORD: &str = "a-correct-horse-password";

/// An ed25519 key from a fixed seed, so a test can make the same key twice.
pub fn ed25519(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

/// A fresh ECDSA P-256 key.
pub fn ecdsa_p256() -> PrivateKey {
    PrivateKey::random(
        &mut rand::rng(),
        Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        },
    )
    .unwrap()
}

/// The private key as the operator would paste it, encrypted with `passphrase` when given.
pub fn openssh_text(key: &PrivateKey, passphrase: Option<&str>) -> String {
    let key = match passphrase {
        Some(passphrase) => key.encrypt(&mut rand::rng(), passphrase).unwrap(),
        None => key.clone(),
    };
    key.to_openssh(LineEnding::LF).unwrap().to_string()
}

/// One authentication request the server received, with no secret in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Attempt {
    None,
    Password { user: String, correct: bool },
    PublicKey { user: String },
}

/// A running server; dropping it stops it.
pub struct SshServer {
    pub port: u16,
    attempts: Arc<Mutex<Vec<Attempt>>>,
    task: JoinHandle<()>,
}

impl SshServer {
    /// Serves on a free loopback port with `host_keys`, accepting `PASSWORD` and `client_key`.
    pub async fn start(host_keys: Vec<PrivateKey>, client_key: Option<PublicKey>) -> Self {
        let config = Arc::new(server::Config {
            keys: host_keys,
            auth_rejection_time: Duration::from_millis(10),
            auth_rejection_time_initial: Some(Duration::from_millis(1)),
            inactivity_timeout: Some(Duration::from_secs(20)),
            ..server::Config::default()
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let recorder = Recorder {
            attempts: Arc::clone(&attempts),
            client_key,
        };
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(serve(Arc::clone(&config), stream, recorder.clone()));
            }
        });
        Self {
            port,
            attempts,
            task,
        }
    }

    /// Every authentication attempt so far, in order.
    pub fn attempts(&self) -> Vec<Attempt> {
        self.attempts.lock().unwrap().clone()
    }
}

/// Runs one client's session to its end.
async fn serve(config: Arc<server::Config>, stream: TcpStream, recorder: Recorder) {
    if let Ok(session) = server::run_stream(config, stream, recorder).await {
        let _ended = session.await;
    }
}

impl Drop for SshServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
struct Recorder {
    attempts: Arc<Mutex<Vec<Attempt>>>,
    client_key: Option<PublicKey>,
}

impl Recorder {
    fn record(&self, attempt: Attempt) {
        self.attempts.lock().unwrap().push(attempt);
    }

    fn knows(&self, key: &PublicKey) -> bool {
        self.client_key
            .as_ref()
            .is_some_and(|known| known.key_data() == key.key_data())
    }
}

fn verdict(accept: bool) -> Auth {
    if accept { Auth::Accept } else { Auth::reject() }
}

/// Every hook answers at once, so each returns a ready future.
impl server::Handler for Recorder {
    type Error = russh::Error;

    fn auth_none(&mut self, _user: &str) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.record(Attempt::None);
        std::future::ready(Ok(Auth::reject()))
    }

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        let correct = user == USER && password == PASSWORD;
        self.record(Attempt::Password {
            user: user.to_owned(),
            correct,
        });
        std::future::ready(Ok(verdict(correct)))
    }

    fn auth_publickey_offered(
        &mut self,
        user: &str,
        offer: PublicKeyOffer<'_>,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        self.record(Attempt::PublicKey {
            user: user.to_owned(),
        });
        std::future::ready(Ok(verdict(user == USER && self.knows(offer.public_key))))
    }

    fn auth_publickey(
        &mut self,
        user: &str,
        key: VerifiedPublicKey<'_>,
    ) -> impl Future<Output = Result<Auth, Self::Error>> + Send {
        std::future::ready(Ok(verdict(user == USER && self.knows(key.public_key))))
    }
}
