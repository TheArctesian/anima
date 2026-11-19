//! The pure parts of sources: health, schedule, verdicts, addresses, the App key, the paths of
//! the connect flow and the in-memory token map.

mod account_keys;
mod addresses;
mod health_table;
mod keys;
mod registration_paths;
mod schedule_times;
mod tokens;

use std::sync::LazyLock;

use rsa::{
    RsaPrivateKey,
    pkcs1::{EncodeRsaPrivateKey as _, LineEnding},
    rand_core::OsRng,
};

/// A throwaway 2048-bit RSA key made in this test process, as PKCS#1 PEM. Never a key file.
pub static TEST_KEY: LazyLock<String> = LazyLock::new(|| pem_of_bits(2048));

/// A new RSA key of `bits`, as PKCS#1 PEM.
pub fn pem_of_bits(bits: usize) -> String {
    let key = RsaPrivateKey::new(&mut OsRng, bits).unwrap();
    key.to_pkcs1_pem(LineEnding::LF).unwrap().to_string()
}
