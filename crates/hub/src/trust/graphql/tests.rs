//! `acceptServerCertificate`'s argument rules: the fingerprint the operator pastes may carry
//! surrounding space, in either form or case, and is stored as lower-case colon pairs (SEC-007,
//! Q2-10).

use async_graphql::ID;

use super::certificate_acceptance;
use crate::{fingerprint::CertificateFingerprint, ids::DeviceId};

const DEVICE: &str = "0190d3f2-6c4e-7a1b-8f3e-2b9c4d5e6f70";
const PAIRS: &str = "e3:b0:c4:42:98:fc:1c:14:9a:fb:f4:c8:99:6f:b9:24:\
                     27:ae:41:e4:64:9b:93:4c:a4:95:99:1b:78:52:b8:55";
const HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn sec_007_a_pasted_fingerprint_is_trimmed_in_either_form() {
    let device: DeviceId = DEVICE.parse().unwrap();
    for pasted in [
        format!(" {}\n", PAIRS.to_uppercase()),
        format!("\t{HEX} \n"),
    ] {
        let acceptance = certificate_acceptance(&ID::from(DEVICE), &pasted).unwrap();
        assert_eq!(acceptance.device, device);
        assert_eq!(acceptance.fingerprint, CertificateFingerprint::of(b""));
        assert_eq!(acceptance.fingerprint.colon_pairs(), PAIRS);
    }
}

#[test]
fn sec_007_a_fingerprint_with_space_inside_is_refused_on_its_field() {
    let pasted = PAIRS.replacen(':', ": ", 1);
    let errors = certificate_acceptance(&ID::from(DEVICE), &pasted).unwrap_err();
    let fields: Vec<_> = errors.iter().map(|error| error.field).collect();
    assert_eq!(fields, ["fingerprintSha256"]);
}
