//! The Phase 3 variables: public origin, web root, recording root and limit (K7, D-2, D-3).

use super::tests::env;
use super::*;

#[test]
fn k7_phase_3_variables_default_when_unset() {
    let config = from_env(&env(&[])).unwrap();
    assert_eq!(config.public_origin, "https://hub.anima.ing");
    assert_eq!(config.web_root, None);
    assert_eq!(config.recording_root, None);
    assert_eq!(config.recording_max_bytes, 2_147_483_648);
}

#[test]
fn k7_phase_3_variables_take_explicit_values() {
    let config = from_env(&env(&[
        (PUBLIC_ORIGIN, "https://anima.example.org:8443"),
        (WEB_ROOT, "/usr/share/anima/web"),
        (RECORDING_ROOT, "/var/lib/anima/recordings"),
        (RECORDING_MAX_BYTES, "1048576"),
    ]))
    .unwrap();
    assert_eq!(config.public_origin, "https://anima.example.org:8443");
    assert_eq!(config.web_root, Some(PathBuf::from("/usr/share/anima/web")));
    assert_eq!(
        config.recording_root,
        Some(PathBuf::from("/var/lib/anima/recordings"))
    );
    assert_eq!(config.recording_max_bytes, 1_048_576);
}

#[test]
fn d_2_empty_web_root_and_empty_recording_root_are_none() {
    let config = from_env(&env(&[(WEB_ROOT, ""), (RECORDING_ROOT, "")])).unwrap();
    assert_eq!(config.web_root, None);
    assert_eq!(config.recording_root, None);
}

#[test]
fn d_3_public_origin_accepts_canonical_https_and_loopback_http() {
    for good in [
        "https://hub.anima.ing",
        "https://hub.anima.ing:8443",
        "http://127.0.0.1:3000",
        "http://localhost:8080",
        "http://localhost",
        "http://[::1]:3000",
        "https://[2001:db8::1]",
    ] {
        let config = from_env(&env(&[(PUBLIC_ORIGIN, good)])).unwrap();
        assert_eq!(config.public_origin, good);
    }
}

#[test]
fn d_3_public_origin_refuses_every_non_canonical_form_naming_the_variable() {
    for bad in [
        "https://hub.anima.ing/path",
        "https://hub.anima.ing/",
        "https://hub.anima.ing?x=1",
        "https://hub.anima.ing#top",
        "https://Hub.Anima.ing",
        "https://user@hub.anima.ing",
        "https://hub.anima.ing:443",
        "http://localhost:80",
        "https://hub.anima.ing:0443",
        "https://hub.anima.ing:0",
        "https://hub.anima.ing:",
        "http://hub.anima.ing",
        "http://192.168.1.5:3000",
        "ftp://hub.anima.ing",
        "hub.anima.ing",
        "https://",
        "https://hub..anima.ing",
        "https://[::1",
        "https://[2001:DB8::1]",
    ] {
        let error = from_env(&env(&[(PUBLIC_ORIGIN, bad)])).unwrap_err();
        assert!(
            matches!(&error, Error::Origin { variable, .. } if *variable == PUBLIC_ORIGIN),
            "{bad:?} gave {error:?}"
        );
        assert!(error.to_string().contains(PUBLIC_ORIGIN));
    }
}

#[test]
fn d_3_empty_public_origin_is_refused_naming_the_variable() {
    let error = from_env(&env(&[(PUBLIC_ORIGIN, "")])).unwrap_err();
    assert!(matches!(error, Error::Empty { variable } if variable == PUBLIC_ORIGIN));
}

#[test]
fn k7_recording_max_bytes_must_be_a_positive_integer_naming_the_variable() {
    for bad in ["0", "-5", "lots", "1.5", "", "99999999999999999999999"] {
        let error = from_env(&env(&[(RECORDING_MAX_BYTES, bad)])).unwrap_err();
        let named = match error {
            Error::NotPositive { variable, .. } | Error::Empty { variable } => {
                variable == RECORDING_MAX_BYTES
            }
            _ => false,
        };
        assert!(named, "{bad:?}");
    }
}
