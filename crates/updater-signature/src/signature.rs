//! The updater signature as tauri-plugin-updater reads it (BLD-046, UPD-002): the `.sig` file is
//! base64 of a minisign signature over the BLAKE2b-512 hash of the file, and the trusted comment
//! holds `version:<v>` for `requireSignedVersion`. Pure over the bytes it is given; the time is a
//! parameter (CQ-021).

use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use blake2::{Blake2b512, Digest};
use ed25519_dalek::Signer;

use crate::{
    error::SignatureError,
    key_file::{PublicKey, SecretKey},
};

/// `ED`: the signature is over the BLAKE2b-512 hash of the file (minisign's prehashed mode).
const PREHASHED: &[u8] = b"ED";
const UNTRUSTED: &str = "untrusted comment: signature from tauri secret key";
const TRUSTED_PREFIX: &str = "trusted comment: ";
/// The longest `.sig` text read; one is a few hundred bytes.
const MAX_SIGNATURE_BYTES: usize = 4096;
const CHUNK: usize = 64 * 1024;

/// What one signature names in its trusted comment, as `tauri signer sign` writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signed<'a> {
    /// Seconds since the Unix epoch.
    pub timestamp: u64,
    /// The file's name, without folders. No tab and no line break.
    pub file_name: &'a str,
    pub version: &'a str,
}

/// The BLAKE2b-512 hash minisign signs in prehashed mode, fed in chunks.
#[derive(Default)]
pub struct Prehash(Blake2b512);

impl Prehash {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    #[must_use]
    pub fn finish(self) -> [u8; 64] {
        self.0.finalize().into()
    }
}

fn trusted_comment(signed: &Signed<'_>) -> Result<String, SignatureError> {
    let clean = |text: &str| !text.is_empty() && !text.contains(['\t', '\n', '\r']);
    if !clean(signed.file_name) || !clean(signed.version) {
        return Err(SignatureError::Signature);
    }
    Ok(format!(
        "timestamp:{}\tfile:{}\tversion:{}",
        signed.timestamp, signed.file_name, signed.version
    ))
}

impl SecretKey {
    /// The `.sig` file text for the file whose [`Prehash`] is `digest`: base64 of the four
    /// minisign lines. The same key, digest and `signed` always give the same text.
    ///
    /// # Errors
    ///
    /// `Signature` when a field of `signed` is empty or holds a tab or a line break.
    pub fn sign(&self, digest: &[u8; 64], signed: &Signed<'_>) -> Result<String, SignatureError> {
        let comment = trusted_comment(signed)?;
        let signing = self.signing_key();
        let file_signature = signing.sign(digest).to_bytes();
        let mut global_input = file_signature.to_vec();
        global_input.extend_from_slice(comment.as_bytes());
        let global = signing.sign(&global_input).to_bytes();

        let mut line = PREHASHED.to_vec();
        line.extend_from_slice(&self.public_key().keynum());
        line.extend_from_slice(&file_signature);
        let text = format!(
            "{UNTRUSTED}\n{}\n{TRUSTED_PREFIX}{comment}\n{}\n",
            STANDARD.encode(line),
            STANDARD.encode(global)
        );
        Ok(STANDARD.encode(text))
    }
}

/// A `.sig` text to check with one public key, by the verifier tauri-plugin-updater uses.
pub struct SignatureCheck {
    key: minisign_verify::PublicKey,
    signature: minisign_verify::Signature,
}

impl SignatureCheck {
    /// Reads `signature_file` (base64 of the minisign lines, as a `.sig` file and a feed's
    /// `signature` hold it) for checking with `public`.
    ///
    /// # Errors
    ///
    /// `Signature` when the text is not a minisign signature.
    pub fn new(public: &PublicKey, signature_file: &str) -> Result<Self, SignatureError> {
        let text = signature_file.trim();
        if text.len() > MAX_SIGNATURE_BYTES {
            return Err(SignatureError::Signature);
        }
        let decoded = STANDARD
            .decode(text)
            .map_err(|_| SignatureError::Signature)?;
        let text = String::from_utf8(decoded).map_err(|_| SignatureError::Signature)?;
        let signature =
            minisign_verify::Signature::decode(&text).map_err(|_| SignatureError::Signature)?;
        let key = minisign_verify::PublicKey::from_base64(&public.line())
            .map_err(|_| SignatureError::Signature)?;
        Ok(Self { key, signature })
    }

    /// A stream to feed the signed file to, in chunks.
    ///
    /// # Errors
    ///
    /// `Signature` when the signature is not by this key or not made in prehashed mode, the only
    /// mode the updater accepts.
    pub fn stream(&self) -> Result<SignatureStream<'_>, SignatureError> {
        let verifier = self
            .key
            .verify_stream(&self.signature)
            .map_err(|_| SignatureError::Signature)?;
        Ok(SignatureStream {
            verifier,
            signature: &self.signature,
        })
    }
}

/// The check of one file as it streams.
pub struct SignatureStream<'a> {
    verifier: minisign_verify::StreamVerifier<'a>,
    signature: &'a minisign_verify::Signature,
}

impl SignatureStream<'_> {
    pub fn update(&mut self, chunk: &[u8]) {
        self.verifier.update(chunk);
    }

    /// Checks that the bytes fed are the file the signature signed, then that its trusted
    /// comment names exactly `version` once (`requireSignedVersion`, BLD-046). The comment is
    /// read only after the signature over it holds.
    ///
    /// # Errors
    ///
    /// `Signature` when the signature does not hold; `SignedVersion` when it holds for another
    /// version.
    pub fn finish(mut self, version: &str) -> Result<(), SignatureError> {
        self.verifier
            .finalize()
            .map_err(|_| SignatureError::Signature)?;
        let versions = self
            .signature
            .trusted_comment()
            .split('\t')
            .filter_map(|field| field.strip_prefix("version:"));
        if versions.eq([version]) {
            Ok(())
        } else {
            Err(SignatureError::SignedVersion)
        }
    }
}

/// Checks `signature_file` over `file` with `public` for exactly `version` (BLD-046).
///
/// # Errors
///
/// As [`SignatureCheck::new`] and [`SignatureStream::finish`]; `Read` when `file` fails.
pub fn verify(
    public: &PublicKey,
    mut file: impl Read,
    signature_file: &str,
    version: &str,
) -> Result<(), SignatureError> {
    let check = SignatureCheck::new(public, signature_file)?;
    let mut stream = check.stream()?;
    let mut buffer = vec![0; CHUNK];
    loop {
        let read = file.read(&mut buffer).map_err(|_| SignatureError::Read)?;
        if read == 0 {
            return stream.finish(version);
        }
        stream.update(&buffer[..read]);
    }
}
