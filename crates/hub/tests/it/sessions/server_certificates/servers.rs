//! In-process RDP and VNC servers on the loopback interface for the connection test. The RDP
//! server answers the X.224 request as it is told and, for TLS, presents a throwaway
//! certificate made by rcgen (never a real key), self-signed or with its issuer after it; both
//! record every byte the hub sends.

use std::{sync::Arc, time::Duration};

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, date_time_ymd};
use rustls::{
    ServerConfig,
    crypto::ring,
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};
use tokio_rustls::TlsAcceptor;

/// How the RDP server answers the Connection Request.
#[derive(Clone, Debug)]
pub enum Answer {
    /// Chooses `HYBRID` (NLA), then a TLS handshake with its certificate.
    Tls,
    /// Chooses standard RDP security: no TLS.
    Standard,
    /// An RDP Negotiation Failure with this code.
    Failure(u32),
    /// These bytes, then the connection closes.
    Raw(Vec<u8>),
}

/// What one connection sent the server.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Visit {
    /// The bytes before the server answered: the Connection Request, or the whole visit.
    pub request: Vec<u8>,
    /// Whether the TLS handshake finished.
    pub handshake: bool,
    /// Every byte after the handshake (RDP) or after the banner (VNC), until the hub closed.
    pub after: Vec<u8>,
}

/// A running server; dropping it stops it.
pub struct Server {
    pub port: u16,
    visits: mpsc::UnboundedReceiver<Visit>,
    task: JoinHandle<()>,
}

impl Server {
    /// The next connection's visit, once the hub has closed it.
    pub async fn visit(&mut self) -> Visit {
        timeout(Duration::from_secs(10), self.visits.recv())
            .await
            .expect("a visit within 10 s")
            .expect("the server runs")
    }

    /// Whether any connection reached the server, waiting a moment for one.
    pub async fn was_visited(&mut self) -> bool {
        timeout(Duration::from_millis(300), self.visits.recv())
            .await
            .is_ok()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A throwaway certificate with common name `name`, the DER of the leaf the server presents
/// first.
pub struct Certificate {
    pub der: Vec<u8>,
    acceptor: TlsAcceptor,
}

/// The leaf's parameters: `name` and `name.lan`, valid until 2031.
fn leaf_params(name: &str) -> CertificateParams {
    let mut params = CertificateParams::new(vec![format!("{name}.lan")]).unwrap();
    params.distinguished_name.push(DnType::CommonName, name);
    params.not_after = date_time_ymd(2031, 1, 1);
    params
}

impl Certificate {
    /// A self-signed leaf, presented alone.
    pub fn new(name: &str) -> Self {
        let key = KeyPair::generate().unwrap();
        let leaf = leaf_params(name).self_signed(&key).unwrap();
        Self::serving(vec![leaf.der().to_vec()], &key)
    }

    /// A leaf signed by a throwaway issuer, presented as the chain leaf then issuer.
    pub fn issued(name: &str) -> Self {
        let issuer_key = KeyPair::generate().unwrap();
        let mut issuer_params = CertificateParams::new(Vec::new()).unwrap();
        issuer_params
            .distinguished_name
            .push(DnType::CommonName, "Throwaway Issuer");
        issuer_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let issuer_der = issuer_params
            .self_signed(&issuer_key)
            .unwrap()
            .der()
            .to_vec();
        let issuer = Issuer::new(issuer_params, issuer_key);
        let key = KeyPair::generate().unwrap();
        let leaf = leaf_params(name).signed_by(&key, &issuer).unwrap();
        Self::serving(vec![leaf.der().to_vec(), issuer_der], &key)
    }

    /// A TLS server presenting `chain`, whose first entry is signed for `key`.
    fn serving(chain: Vec<Vec<u8>>, key: &KeyPair) -> Self {
        let der = chain[0].clone();
        let chain = chain.into_iter().map(Into::into).collect();
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key.serialize_der()));
        let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(chain, key)
            .unwrap();
        Self {
            der,
            acceptor: TlsAcceptor::from(Arc::new(config)),
        }
    }

    /// SHA-256 of the DER as the hub must show it, computed here, not by the hub.
    pub fn fingerprint(&self) -> String {
        let pairs: Vec<String> = Sha256::digest(&self.der)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        pairs.join(":")
    }
}

fn confirm(kind: u8, value: u32) -> Vec<u8> {
    let mut bytes = vec![3, 0, 0, 19, 14, 0xd0, 0, 0, 0x12, 0x34, 0, kind, 0, 8, 0];
    bytes.extend_from_slice(&value.to_le_bytes());
    bytes
}

/// Every byte until the peer closes, or until 5 s pass.
async fn rest(mut stream: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ended = timeout(Duration::from_secs(5), stream.read_to_end(&mut bytes)).await;
    bytes
}

async fn rdp_visit(mut stream: TcpStream, answer: Answer, acceptor: TlsAcceptor) -> Visit {
    let mut request = vec![0; 19];
    if stream.read_exact(&mut request).await.is_err() {
        return Visit::default();
    }
    let reply = match &answer {
        Answer::Tls => confirm(2, 0x2),
        Answer::Standard => confirm(2, 0),
        Answer::Failure(code) => confirm(3, *code),
        Answer::Raw(bytes) => bytes.clone(),
    };
    let _sent = stream.write_all(&reply).await;
    match answer {
        Answer::Tls => {}
        Answer::Raw(_) => {
            return Visit {
                request,
                ..Visit::default()
            };
        }
        Answer::Standard | Answer::Failure(_) => {
            return Visit {
                request,
                after: rest(stream).await,
                ..Visit::default()
            };
        }
    }
    match acceptor.accept(stream).await {
        Ok(tls) => Visit {
            request,
            handshake: true,
            after: rest(tls).await,
        },
        Err(_) => Visit {
            request,
            ..Visit::default()
        },
    }
}

/// Serves `run` for every connection on a free loopback port.
async fn serve<F, Fut>(run: F) -> Server
where
    F: Fn(TcpStream) -> Fut + Send + 'static,
    Fut: Future<Output = Visit> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sender, visits) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let (sender, visit) = (sender.clone(), run(stream));
            tokio::spawn(async move { sender.send(visit.await) });
        }
    });
    Server { port, visits, task }
}

/// An RDP server that answers `answer`, presenting `certificate` for TLS.
pub async fn rdp(answer: Answer, certificate: &Certificate) -> Server {
    let acceptor = certificate.acceptor.clone();
    serve(move |stream| rdp_visit(stream, answer.clone(), acceptor.clone())).await
}

/// A VNC server that sends `banner` first.
pub async fn vnc(banner: &'static [u8]) -> Server {
    serve(move |mut stream| async move {
        let _sent = stream.write_all(banner).await;
        Visit {
            after: rest(stream).await,
            ..Visit::default()
        }
    })
    .await
}
