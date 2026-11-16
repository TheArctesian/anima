//! An RDP device on a loopback port for the opening and session tests, on `IronRDP`'s
//! acceptor: TLS with a certificate made at start, NLA with one account when the test asks
//! for it, and a screen of one size. It serves every connection it gets until it is
//! dropped, tells the test what each client sent, and paints when the test tells it.
//!
//! The acceptor's futures are not `Send`, so the device runs on its own thread with a
//! runtime of its own; the test talks to it through channels.

mod connection;
mod input;
mod screen;

use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use anima_gateway::rdp::{KeyEvent, PointerEvent};
use anima_gateway::screen::Area;
use rustls::ServerConfig;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest as _, Sha256};
use tokio::sync::{mpsc, oneshot, watch};

/// How long a test waits for the device, or for the client's side of a session.
pub const WAIT: Duration = Duration::from_secs(10);

/// The security the device offers.
#[derive(Clone, Copy)]
pub enum Offered {
    /// TLS only; the client signs in with its Client Info, which the device notes.
    Tls,
    /// NLA only, with the one account it accepts.
    Nla {
        username: &'static str,
        password: &'static str,
    },
}

/// What the device is.
#[derive(Clone, Copy)]
pub struct Setup {
    /// The size of its screen, in pixels.
    pub size: (u16, u16),
    pub offered: Offered,
}

/// What the device saw a client send, in order.
#[derive(Debug, PartialEq, Eq)]
pub enum Seen {
    /// The account of a TLS sign-in, from the Client Info PDU.
    SignIn {
        username: String,
        password: String,
        domain: Option<String>,
    },
    Key(KeyEvent),
    Pointer {
        x: u16,
        y: u16,
        event: PointerEvent,
    },
    /// A Synchronize event: the client's lock keys, as RDP's flag bits.
    Locks(u8),
    /// The connection ended, for any reason.
    Ended,
}

/// What the test tells the connected client's connection to do.
enum Tell {
    Paint { area: Area, colour: [u8; 3] },
    Resize { width: u16, height: u16 },
    Close,
}

/// What every connection of one device shares.
struct Served {
    setup: Setup,
    tls: Arc<ServerConfig>,
    /// The key of the certificate, which NLA binds the sign-in to.
    public_key: Vec<u8>,
    seen: mpsc::Sender<Seen>,
    /// The connection whose session is active, the one the test's tells go to.
    active: watch::Sender<Option<mpsc::Sender<Tell>>>,
}

pub struct Device {
    pub address: SocketAddr,
    /// The SHA-256 of its certificate's DER bytes.
    pub fingerprint: [u8; 32],
    seen: mpsc::Receiver<Seen>,
    active: watch::Receiver<Option<mpsc::Sender<Tell>>>,
    /// Dropping it ends the device's thread, its listener and its connections.
    _stop: oneshot::Sender<()>,
}

impl Device {
    /// Starts the device on a loopback port.
    ///
    /// # Panics
    /// When the test's computer gives no port, thread or certificate.
    #[must_use]
    pub fn start(setup: Setup) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        listener
            .set_nonblocking(true)
            .expect("a listener for tokio");
        let address = listener.local_addr().expect("an address");
        let made = rcgen::generate_simple_self_signed(vec!["anima-test-device".to_owned()])
            .expect("a certificate");
        let certificate = made.cert.der().clone();
        let fingerprint = Sha256::digest(&certificate).into();
        let (_, parsed) = x509_parser::parse_x509_certificate(&certificate).expect("parsed");
        let public_key = parsed.public_key().subject_public_key.data.to_vec();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(made.signing_key.serialize_der()));
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut tls = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("TLS versions")
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key)
            .expect("TLS settings");
        // NLA does not take a resumed TLS session; the client never asks for one.
        tls.send_tls13_tickets = 0;

        let (seen_by_device, seen) = mpsc::channel(64);
        let (active_now, active) = watch::channel(None);
        let (stop, stopped) = oneshot::channel();
        let served = Served {
            setup,
            tls: Arc::new(tls),
            public_key,
            seen: seen_by_device,
            active: active_now,
        };
        std::thread::spawn(move || run(listener, served, stopped));
        Self {
            address,
            fingerprint,
            seen,
            active,
            _stop: stop,
        }
    }

    /// The next thing the device saw.
    ///
    /// # Panics
    /// When the device saw nothing within [`WAIT`], which fails the test.
    pub async fn next(&mut self) -> Seen {
        tokio::time::timeout(WAIT, self.seen.recv())
            .await
            .expect("the device saw something in time")
            .expect("the device runs")
    }

    /// Fills `area` of its screen with `colour` (red, green, blue) and sends it to the
    /// client of the active session as uncompressed bitmaps.
    pub async fn paint(&mut self, area: Area, colour: [u8; 3]) {
        self.tell(Tell::Paint { area, colour }).await;
    }

    /// Changes the size of its screen: Deactivate All, then a new capability exchange.
    pub async fn resize(&mut self, width: u16, height: u16) {
        self.tell(Tell::Resize { width, height }).await;
    }

    /// Closes the connection of the active session.
    pub async fn close(&mut self) {
        self.tell(Tell::Close).await;
    }

    async fn tell(&mut self, tell: Tell) {
        let active = tokio::time::timeout(WAIT, self.active.wait_for(Option::is_some))
            .await
            .expect("a session is active in time")
            .expect("the device runs")
            .clone()
            .expect("an active session");
        active.send(tell).await.expect("the session takes it");
    }
}

/// The device's thread: it accepts connections until the test drops the device.
fn run(listener: std::net::TcpListener, served: Served, stopped: oneshot::Receiver<()>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    let connections = tokio::task::LocalSet::new();
    connections.block_on(&runtime, async move {
        let listener = tokio::net::TcpListener::from_std(listener).expect("a listener");
        let served = Rc::new(served);
        let accepting = async {
            while let Ok((client, _)) = listener.accept().await {
                tokio::task::spawn_local(connection::serve(client, Rc::clone(&served)));
            }
        };
        tokio::select! {
            () = accepting => {}
            _ = stopped => {}
        }
    });
}
