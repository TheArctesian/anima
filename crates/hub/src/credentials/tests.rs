//! Unit tests for the credentials crypto, grouped by concept; shared fixtures live here.

mod disclosure;
mod host;
mod key;
mod layout;
mod password;
mod sealing;
mod ssh;

use zeroize::Zeroizing;

use super::{Secret, Vault};

/// Row context used by tests that do not exercise context binding.
const CONTEXT: &[u8] = b"test/context";

/// Bytes 0x00..=0x1f: a fixed throwaway test key.
fn test_key() -> [u8; 32] {
    std::array::from_fn(|index| u8::try_from(index).unwrap())
}

fn vault_from(key: [u8; 32]) -> Vault {
    Vault::from_key(Zeroizing::new(key))
}

fn other_vault() -> Vault {
    vault_from([0xa5; 32])
}

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}
