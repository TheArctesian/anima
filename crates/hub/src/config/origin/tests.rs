//! Origin spelling rules: only the exact form a browser sends in `Origin` is accepted (D-3).

use std::collections::BTreeMap;

use super::super::{Error, PUBLIC_ORIGIN, from_env};

fn env(origin: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(PUBLIC_ORIGIN.to_owned(), origin.to_owned())])
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
        let config = from_env(&env(good)).unwrap();
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
        "https://127.1",
        "https://1234",
        "https://0x7f000001",
        "https://010.0.0.1",
        "https://256.1.1.1",
        "https://hub.123",
        "https://hub.anima.ing.",
        "https://-hub.anima.ing",
        "https://hub-.anima.ing",
        "https://[::ffff:127.0.0.1]",
        "https://[::ffff:7f00:1]",
        "https://[::127.0.0.1]",
    ] {
        let error = from_env(&env(bad)).unwrap_err();
        assert!(
            matches!(&error, Error::Origin { variable, .. } if *variable == PUBLIC_ORIGIN),
            "{bad:?} gave {error:?}"
        );
        assert!(error.to_string().contains(PUBLIC_ORIGIN));
    }
}

#[test]
fn d_3_empty_public_origin_is_refused_naming_the_variable() {
    let error = from_env(&env("")).unwrap_err();
    assert!(matches!(error, Error::Empty { variable } if variable == PUBLIC_ORIGIN));
}

#[test]
fn d_3_dotted_quad_ipv4_and_length_limits_are_judged_by_the_dns_rules() {
    assert!(from_env(&env("https://192.0.2.7")).is_ok());
    let label = "a".repeat(63);
    assert!(from_env(&env(&format!("https://{label}.example"))).is_ok());
    let too_long_label = "a".repeat(64);
    assert!(from_env(&env(&format!("https://{too_long_label}.example"))).is_err());
    let long_host = vec![label; 4].join(".");
    assert!(from_env(&env(&format!("https://{long_host}.example"))).is_err());
}

#[test]
fn log_007_origin_error_does_not_echo_a_password_in_the_userinfo() {
    let error = from_env(&env("https://op:hunter2@hub.anima.ing")).unwrap_err();
    assert!(matches!(error, Error::Origin { .. }));
    assert!(!error.to_string().contains("hunter2"));
    assert!(!format!("{error:?}").contains("hunter2"));
}

#[test]
fn log_007_origin_error_hides_userinfo_even_without_a_scheme_separator() {
    for bad in ["op:hunter2@hub.anima.ing", "https:/op:hunter2@hub"] {
        let error = from_env(&env(bad)).unwrap_err();
        assert!(matches!(error, Error::Origin { .. }));
        assert!(!error.to_string().contains("hunter2"), "{bad}");
        assert!(!format!("{error:?}").contains("hunter2"), "{bad}");
    }
}
