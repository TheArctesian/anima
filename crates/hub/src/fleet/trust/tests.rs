//! Unit tests of the pure trust rules and of the store's crate-private writes. Public
//! behaviour is tested through GraphQL in `tests/it/sessions/host_keys` and
//! `tests/it/sessions/server_certificates`.

mod certificate;
mod observations;
mod public_key;
mod store;

/// Two real OpenSSH public keys (throwaway, made with `ssh-keygen`; the private halves were
/// deleted) and the fingerprints `ssh-keygen -l` printed for them.
pub(crate) const ED25519: &str =
    "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
pub(crate) const ED25519_FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";
pub(crate) const ECDSA: &str = "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBN6Al4EgiR+faDnuIVjWmW1yCwStnaO+iRCpsRc7Py09CTCdHczEiAwuagxLahrlBz/ZDfvgKxIt9rS8Rv3xTDM=";
const ECDSA_FINGERPRINT: &str = "SHA256:5B0BaMRWDnvpdw0F6yEj4v4XY3LfY9f26fSNr+SrSgQ";

/// A deterministic xorshift generator, so the parser fuzz runs the same inputs every time.
pub(crate) struct Inputs(pub(crate) u64);

impl Inputs {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }

    pub(crate) fn bytes(&mut self, max_len: usize) -> Vec<u8> {
        let len = self.below(max_len + 1);
        (0..len).map(|_| self.next().to_le_bytes()[0]).collect()
    }

    /// Text over `alphabet`, up to `max_len` characters.
    pub(crate) fn text(&mut self, alphabet: &[u8], max_len: usize) -> String {
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| char::from(alphabet[self.below(alphabet.len())]))
            .collect()
    }
}
