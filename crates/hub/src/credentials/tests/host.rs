//! The host sealed inside a secret (SEC-011): the frame, the comparison, and the two contexts
//! that keep host-bound rows and rows from before apart.

use proptest::prelude::*;

use super::{secret, test_key, vault_from};
use crate::credentials::{
    Error, Sealed, Secret, Vault,
    host::{self, frame, permits, unframe},
    store::StoredSecret,
};
use crate::ids::DeviceId;

const DEVICE: &str = "0190a000-0000-7000-8002-00000000000a";
const KIND: &str = "vnc_password";

fn device() -> DeviceId {
    DEVICE.parse().unwrap()
}

/// The context of a row sealed before SEC-011.
fn legacy() -> Vec<u8> {
    format!("credentials.secrets:{DEVICE}:{KIND}").into_bytes()
}

fn row(sealed: &Sealed, legacy_host: Option<&str>) -> StoredSecret {
    StoredSecret {
        kind: KIND.to_owned(),
        nonce: sealed.nonce().to_vec(),
        ciphertext: sealed.ciphertext().to_vec(),
        key_id: sealed.key_id().to_owned(),
        legacy_host: legacy_host.map(str::to_owned),
    }
}

fn open(vault: &Vault, row: StoredSecret, to: &str) -> Result<Secret, Error> {
    host::open(vault, row, (device(), to), &legacy())
}

#[test]
fn sec_011_context_names_the_device_and_kind_under_its_own_prefix() {
    assert_eq!(
        host::context(device(), KIND),
        b"credentials.secrets.host:0190a000-0000-7000-8002-00000000000a:vnc_password"
    );
}

#[test]
fn sec_011_the_host_text_counts_but_not_its_letter_case() {
    assert!(permits("NAS.home.lan", "nas.HOME.lan"));
    assert!(permits("10.10.0.21", "10.10.0.21"));
    assert!(!permits("10.10.0.21", "10.10.0.210"));
    assert!(!permits("nas.home.lan", "nas.home.lan."));
    assert!(!permits("nas.home.lan", "203.0.113.9"));
}

#[test]
fn sec_011_a_secret_opens_only_for_the_host_it_was_entered_for() {
    let vault = vault_from(test_key());
    let sealed = host::seal(&vault, (device(), "NAS.home.lan"), KIND, &secret("pw")).unwrap();
    let opened = open(&vault, row(&sealed, None), "nas.home.lan").unwrap();
    assert_eq!(opened.expose(), b"pw");
    assert!(matches!(
        open(&vault, row(&sealed, None), "attacker.example"),
        Err(Error::EnteredForOtherHost)
    ));
}

#[test]
fn sec_011_a_row_from_before_opens_only_for_its_recorded_host() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("pw"), &legacy()).unwrap();
    let opened = open(&vault, row(&sealed, Some("10.10.0.21")), "10.10.0.21").unwrap();
    assert_eq!(opened.expose(), b"pw");
    assert!(matches!(
        open(&vault, row(&sealed, Some("10.10.0.21")), "10.10.0.99"),
        Err(Error::EnteredForOtherHost)
    ));
}

/// A database writer cannot rebind a host-bound row by giving it a recorded host, nor make a
/// row from before skip its recorded host by clearing it: neither opens under the other
/// context.
#[test]
fn sec_011_the_legacy_column_cannot_rebind_a_row() {
    let vault = vault_from(test_key());
    let bound = host::seal(&vault, (device(), "10.10.0.21"), KIND, &secret("pw")).unwrap();
    assert!(matches!(
        open(
            &vault,
            row(&bound, Some("attacker.example")),
            "attacker.example"
        ),
        Err(Error::Decrypt)
    ));
    let old = vault.seal(&secret("pw"), &legacy()).unwrap();
    assert!(matches!(
        open(&vault, row(&old, None), "attacker.example"),
        Err(Error::Decrypt)
    ));
}

#[test]
fn sec_011_a_truncated_frame_is_malformed() {
    for plain in [&b""[..], b"\x00", b"\x00\x05abc", b"\xff\xff"] {
        assert!(matches!(unframe(plain), Err(Error::Malformed)), "{plain:?}");
    }
    assert!(matches!(unframe(b"\x00\x01\xff"), Err(Error::Malformed)));
}

proptest! {
    /// Any host and secret come back as they went in.
    #[test]
    fn sec_011_a_frame_round_trips(
        host in ".{0,300}",
        bytes in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        let framed = frame(&host, &Secret::new(bytes.clone())).unwrap();
        let (bound, secret) = unframe(framed.expose()).unwrap();
        prop_assert_eq!(bound, host.as_str());
        prop_assert_eq!(secret.expose(), bytes.as_slice());
    }

    /// Whatever bytes a frame holds, reading it never panics, and what it reads fits in them.
    #[test]
    fn sec_011_any_bytes_give_a_frame_or_malformed(
        plain in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        match unframe(&plain) {
            Ok((bound, secret)) => {
                prop_assert_eq!(2 + bound.len() + secret.expose().len(), plain.len());
            }
            Err(error) => prop_assert!(matches!(error, Error::Malformed)),
        }
    }
}
