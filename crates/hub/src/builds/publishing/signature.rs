//! The updater signature of a published file, as tauri-plugin-updater checks it (BLD-064,
//! UPD-002): the feed's `signature` is base64 of a minisign `.sig` file, made in prehashed mode
//! over the file, with `version:<v>` in its trusted comment. Pure over the bytes it is given.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature, StreamVerifier};

/// The public updater key: the same value as `plugins.updater.pubkey` in `tauri.conf.json`,
/// base64 of the minisign public key file (R-108). Public, so it may be printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdaterKey(PublicKey);

impl UpdaterKey {
    /// The key in `text`, or `None` when it is not one.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let decoded = STANDARD.decode(text.trim()).ok()?;
        let file = String::from_utf8(decoded).ok()?;
        PublicKey::decode(&file).ok().map(Self)
    }

    /// A verifier for the file `signature` claims to sign, when it is this key's and made in
    /// prehashed mode, the only mode the updater accepts for a stream.
    pub(super) fn verifier<'a>(&'a self, signature: &'a Signature) -> Option<StreamVerifier<'a>> {
        self.0.verify_stream(signature).ok()
    }
}

/// The signature in a feed entry's `signature` text, or `None` when it is not one.
pub(super) fn signature_of(text: &str) -> Option<Signature> {
    let decoded = STANDARD.decode(text.trim()).ok()?;
    Signature::decode(&String::from_utf8(decoded).ok()?).ok()
}

/// Whether the file fed to `verifier` is the one `signature` signed, and the signature names
/// exactly `version` in its trusted comment (`requireSignedVersion`). The comment is read only
/// once the signature over it holds.
pub(super) fn holds(
    mut verifier: StreamVerifier<'_>,
    signature: &Signature,
    version: &str,
) -> bool {
    verifier.finalize().is_ok()
        && signature
            .trusted_comment()
            .split('\t')
            .filter_map(|field| field.strip_prefix("version:"))
            .eq([version])
}
