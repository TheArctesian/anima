//! The hub's TLS certificate: read from its PEM files, checked against its key, and served
//! to every handshake until a reload swaps it (SEC-001, R-021 H-2).
//!
//! Errors are [`Fault`] kinds only. Parser errors are dropped unread, because their text can
//! quote a line of the file, and the key file holds the private key (LOG-007).

use std::{
    fmt,
    fs::File,
    io::{self, Read as _},
    path::{Path, PathBuf},
    sync::{Arc, PoisonError, RwLock},
};

use tokio_rustls::rustls::{
    self,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject as _},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use zeroize::Zeroizing;

/// Larger certificate or key files are refused unread past this many bytes.
const MAX_FILE_BYTES: usize = 1024 * 1024;

/// The certificate chain and private key files (`ANIMA_TLS_CERT_FILE`, `ANIMA_TLS_KEY_FILE`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsFiles {
    pub cert: PathBuf,
    pub key: PathBuf,
}

/// Why a certificate could not be loaded, by kind. Holds no file content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("the certificate file cannot be read ({0})")]
    CertificateFile(io::ErrorKind),
    #[error("the key file cannot be read ({0})")]
    KeyFile(io::ErrorKind),
    #[error("the certificate file is larger than 1 MiB")]
    CertificateTooLarge,
    #[error("the key file is larger than 1 MiB")]
    KeyTooLarge,
    #[error("the certificate file holds no PEM certificate")]
    NoCertificate,
    #[error("the certificate file is not valid PEM")]
    CertificatePem,
    #[error("the certificate cannot be parsed")]
    CertificateInvalid,
    #[error("the key file holds no valid PEM private key")]
    KeyPem,
    #[error("the private key is of a type the hub cannot use")]
    KeyUnsupported,
    #[error("the private key does not belong to the certificate")]
    KeyMismatch,
    #[error("the certificate files could not be read in time")]
    Interrupted,
}

impl Fault {
    /// A fixed word for the process log.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::CertificateFile(_) => "certificate_file",
            Self::KeyFile(_) => "key_file",
            Self::CertificateTooLarge => "certificate_too_large",
            Self::KeyTooLarge => "key_too_large",
            Self::NoCertificate => "no_certificate",
            Self::CertificatePem => "certificate_pem",
            Self::CertificateInvalid => "certificate_invalid",
            Self::KeyPem => "key_pem",
            Self::KeyUnsupported => "key_unsupported",
            Self::KeyMismatch => "key_mismatch",
            Self::Interrupted => "interrupted",
        }
    }
}

/// The two files' bytes. The key bytes are wiped when dropped.
pub(super) struct Pem {
    pub cert: Vec<u8>,
    pub key: Zeroizing<Vec<u8>>,
}

/// Reads both files, each up to [`MAX_FILE_BYTES`].
pub(super) fn read(files: &TlsFiles) -> Result<Pem, Fault> {
    let cert = read_limited(&files.cert, Fault::CertificateFile)
        .map_err(|fault| fault.unwrap_or(Fault::CertificateTooLarge))?;
    let key = read_limited(&files.key, Fault::KeyFile)
        .map_err(|fault| fault.unwrap_or(Fault::KeyTooLarge))?;
    Ok(Pem {
        cert: cert.to_vec(),
        key,
    })
}

/// The file's bytes in a buffer that never reallocates (so no unwiped copy is left behind);
/// `Err(None)` when the file is too large.
fn read_limited(
    path: &Path,
    unreadable: fn(io::ErrorKind) -> Fault,
) -> Result<Zeroizing<Vec<u8>>, Option<Fault>> {
    let fault = |error: io::Error| Some(unreadable(error.kind()));
    let file = File::open(path).map_err(fault)?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_FILE_BYTES + 1));
    let limit = u64::try_from(MAX_FILE_BYTES + 1).unwrap_or(u64::MAX);
    file.take(limit).read_to_end(&mut bytes).map_err(fault)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(None);
    }
    Ok(bytes)
}

/// The certificate chain with its private key, if the key loads and belongs to the first
/// certificate.
pub(super) fn certified_key(pem: &Pem, provider: &CryptoProvider) -> Result<CertifiedKey, Fault> {
    let chain = CertificateDer::pem_slice_iter(&pem.cert)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Fault::CertificatePem)?;
    if chain.is_empty() {
        return Err(Fault::NoCertificate);
    }
    let key = PrivateKeyDer::from_pem_slice(&pem.key).map_err(|_| Fault::KeyPem)?;
    CertifiedKey::from_der(chain, key, provider).map_err(|error| match error {
        rustls::Error::InconsistentKeys(_) => Fault::KeyMismatch,
        rustls::Error::InvalidCertificate(_) | rustls::Error::NoCertificatesPresented => {
            Fault::CertificateInvalid
        }
        _ => Fault::KeyUnsupported,
    })
}

/// The certificate every new handshake gets.
pub(super) struct Current(RwLock<Arc<CertifiedKey>>);

impl Current {
    pub(super) fn new(key: CertifiedKey) -> Self {
        Self(RwLock::new(Arc::new(key)))
    }

    pub(super) fn get(&self) -> Arc<CertifiedKey> {
        Arc::clone(&self.0.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Serves `key` from the next handshake on; open connections keep theirs.
    pub(super) fn swap(&self, key: CertifiedKey) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(key);
    }
}

/// Shows nothing of the certificate or key (LOG-007).
impl fmt::Debug for Current {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Current(..)")
    }
}

impl ResolvesServerCert for Current {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.get())
    }
}
