//! The updater signature as tauri-plugin-updater reads it (BLD-046, UPD-002): the `.sig` file is
//! base64 of a minisign signature over the BLAKE2b-512 hash of the file, and the trusted comment
//! holds `version:<v>` for `requireSignedVersion`. Pure over a reader; the time is a parameter
//! (CQ-021).

use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use blake2::{Blake2b512, Digest};
use ed25519_dalek::Signer;

use super::{
    error::UpdaterKeyError,
    secret::{PublicKey, SecretKey},
};

/// `ED`: the signature is over the BLAKE2b-512 hash of the file (minisign's prehashed mode).
const PREHASHED: &[u8] = b"ED";
const UNTRUSTED: &str = "untrusted comment: signature from tauri secret key";
const TRUSTED_PREFIX: &str = "trusted comment: ";
const CHUNK: usize = 64 * 1024;

/// What one signature names in its trusted comment, as `tauri signer sign --app-version` writes it.
pub(crate) struct Signed<'a> {
    /// Seconds since the Unix epoch.
    pub timestamp: u64,
    /// The file's name, without folders. No tab and no line break.
    pub file_name: &'a str,
    pub version: &'a str,
}

fn trusted_comment(signed: &Signed<'_>) -> Result<String, UpdaterKeyError> {
    let clean = |text: &str| !text.is_empty() && !text.contains(['\t', '\n', '\r']);
    if !clean(signed.file_name) || !clean(signed.version) {
        return Err(UpdaterKeyError::Signature);
    }
    Ok(format!(
        "timestamp:{}\tfile:{}\tversion:{}",
        signed.timestamp, signed.file_name, signed.version
    ))
}

fn prehash(mut file: impl Read) -> Result<[u8; 64], UpdaterKeyError> {
    let mut hash = Blake2b512::new();
    let mut buffer = vec![0; CHUNK];
    loop {
        let read = file.read(&mut buffer).map_err(|_| UpdaterKeyError::File)?;
        if read == 0 {
            return Ok(hash.finalize().into());
        }
        hash.update(&buffer[..read]);
    }
}

/// The `.sig` file text for `file`: base64 of the four minisign lines.
pub(crate) fn sign(
    key: &SecretKey,
    file: impl Read,
    signed: &Signed<'_>,
) -> Result<String, UpdaterKeyError> {
    let comment = trusted_comment(signed)?;
    let signing = key.signing_key();
    let file_signature = signing.sign(&prehash(file)?).to_bytes();
    let mut global_input = file_signature.to_vec();
    global_input.extend_from_slice(comment.as_bytes());
    let global = signing.sign(&global_input).to_bytes();

    let mut line = PREHASHED.to_vec();
    line.extend_from_slice(&key.public_key().keynum());
    line.extend_from_slice(&file_signature);
    let text = format!(
        "{UNTRUSTED}\n{}\n{TRUSTED_PREFIX}{comment}\n{}\n",
        STANDARD.encode(line),
        STANDARD.encode(global)
    );
    Ok(STANDARD.encode(text))
}

/// The `version:` field of a verified trusted comment, as the updater and `signed-version.ts` read it.
fn signed_version(trusted_comment: &str) -> Option<&str> {
    trusted_comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
}

/// Checks `signature_file` over `file` with `public`, then that it was made for exactly `version`
/// (`assertSignedVersion`, BLD-046). The trusted comment is read only after the signature holds.
pub(crate) fn verify(
    public: &PublicKey,
    mut file: impl Read,
    signature_file: &str,
    version: &str,
) -> Result<(), UpdaterKeyError> {
    let decoded = STANDARD
        .decode(signature_file.trim())
        .map_err(|_| UpdaterKeyError::Signature)?;
    let text = String::from_utf8(decoded).map_err(|_| UpdaterKeyError::Signature)?;
    let signature =
        minisign_verify::Signature::decode(&text).map_err(|_| UpdaterKeyError::Signature)?;
    let key = minisign_verify::PublicKey::from_base64(&public.line())
        .map_err(|_| UpdaterKeyError::Signature)?;
    let mut stream = key
        .verify_stream(&signature)
        .map_err(|_| UpdaterKeyError::Signature)?;
    let mut buffer = vec![0; CHUNK];
    loop {
        let read = file.read(&mut buffer).map_err(|_| UpdaterKeyError::File)?;
        if read == 0 {
            break;
        }
        stream.update(&buffer[..read]);
    }
    stream.finalize().map_err(|_| UpdaterKeyError::Signature)?;
    if signed_version(signature.trusted_comment()) == Some(version) {
        Ok(())
    } else {
        Err(UpdaterKeyError::SignedVersion)
    }
}
