//! The pure parts of sources: health, schedule, verdicts, addresses and the App key.

mod addresses;
mod health_table;
mod keys;
mod schedule_times;

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
