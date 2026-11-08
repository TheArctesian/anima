//! The hub's side of the key exchange (SEC-007, K4 Section 8.3): only the expected key lets
//! the connection go on, and a certificate never does, not even one for the expected key.

use russh::{
    client::Handler as _,
    keys::{
        PrivateKey, PublicKeyOrCertificate,
        ssh_key::{
            certificate::{Builder, CertType},
            private::Ed25519Keypair,
        },
    },
};

use crate::{ssh::Observer, trust::PublicKey};

fn private(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

fn stored(key: &PrivateKey) -> PublicKey {
    PublicKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap()
}

/// What the observer expecting `expected` says to `presented`, and the key it reports.
async fn checked(
    expected: &PrivateKey,
    presented: &PublicKeyOrCertificate,
) -> (bool, Option<PublicKey>) {
    let (mut observer, seen) = Observer::expecting(Some(stored(expected)));
    let go_on = observer.check_server_key(presented).await.unwrap();
    (go_on, seen.await.unwrap())
}

#[tokio::test]
async fn sec_007_the_expected_key_goes_on_and_is_reported() {
    let device = private(1);
    let presented = PublicKeyOrCertificate::from(device.public_key().clone());
    assert_eq!(
        checked(&device, &presented).await,
        (true, Some(stored(&device)))
    );
}

#[tokio::test]
async fn sec_007_another_key_stops_the_connection_and_is_reported() {
    let (device, other) = (private(1), private(2));
    let presented = PublicKeyOrCertificate::from(other.public_key().clone());
    assert_eq!(
        checked(&device, &presented).await,
        (false, Some(stored(&other)))
    );
}

#[tokio::test]
async fn k4_8_3_a_certificate_for_the_expected_key_counts_as_another_key() {
    let (device, authority) = (private(1), private(3));
    let mut builder = Builder::new(vec![0; 16], device.public_key().clone(), 0, u64::MAX).unwrap();
    builder.cert_type(CertType::Host).unwrap();
    builder.key_id("docker-host").unwrap();
    builder.valid_principal("docker-host").unwrap();
    let certificate = builder.sign(&authority).unwrap();
    let presented = PublicKeyOrCertificate::Certificate(certificate);
    assert_eq!(checked(&device, &presented).await, (false, None));
}
