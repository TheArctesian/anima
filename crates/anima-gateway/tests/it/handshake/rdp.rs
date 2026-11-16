//! `select rdp`: the hub's values become a typed opening, and a session that would not
//! check the device's certificate is refused (SEC-007).

use anima_gateway::handshake::{Certificate, Desktop, Opening, Rdp, Refused, Secret, Security};

use super::{hub_opens, hub_values};

const FINGERPRINT: &str = "sha256:00:01:02:03:04:05:06:07:08:09:0a:0b:0c:0d:0e:0f:\
10:11:12:13:14:15:16:17:18:19:1A:1B:1C:1D:1E:1F";

fn opens(values: &[(&str, &str)]) -> Result<Opening, Refused> {
    hub_opens("rdp", hub_values(values)).map(|(opening, _)| opening)
}

fn rdp(values: &[(&str, &str)]) -> Rdp {
    match opens(values).expect("opens").desktop {
        Desktop::Rdp(rdp) => rdp,
        Desktop::Vnc => panic!("an RDP session"),
    }
}

#[test]
fn the_hubs_handshake_opens_an_rdp_session_with_the_accepted_certificate() {
    let opening = opens(&[
        ("username", "operator"),
        ("password", super::PASSWORD),
        ("domain", "LAB"),
        ("security", "nla"),
        ("cert-fingerprints", FINGERPRINT),
        ("resize-method", "display-update"),
        ("disable-paste", "true"),
        // Names the gateway does not offer: the hub sends nothing for them.
        ("enable-drive", "true"),
        ("drive-path", "/drive/1"),
    ])
    .expect("opens");

    assert_eq!(opening.device.to_string(), "192.0.2.7:5900");
    assert_eq!(
        opening.username.as_ref().map(Secret::expose),
        Some("operator")
    );
    assert_eq!(
        opening.password.as_ref().map(Secret::expose),
        Some(super::PASSWORD)
    );
    assert!(opening.copy && !opening.paste);
    let Desktop::Rdp(rdp) = opening.desktop else {
        panic!("an RDP session")
    };
    let accepted: [u8; 32] = std::array::from_fn(|at| u8::try_from(at).expect("small"));
    assert_eq!(
        rdp,
        Rdp {
            domain: Some("LAB".to_owned()),
            security: Security::Nla,
            certificate: Certificate::Accepted(accepted),
            resize: true,
        }
    );
}

#[test]
fn each_security_setting_is_read_and_the_default_is_nla() {
    let with = |security| rdp(&[("security", security), ("ignore-cert", "true")]).security;

    assert_eq!(with(""), Security::Nla);
    assert_eq!(with("nla"), Security::Nla);
    assert_eq!(with("nla-ext"), Security::Nla);
    assert_eq!(with("tls"), Security::Tls);
    assert_eq!(with("any"), Security::TlsOrNla);
    assert!(Security::Nla.may_be_nla() && Security::TlsOrNla.may_be_nla());
    assert!(!Security::Tls.may_be_nla());
}

#[test]
fn security_without_a_certificate_is_refused_as_unsupported() {
    for security in ["rdp", "vmconnect"] {
        let refused = opens(&[("security", security), ("ignore-cert", "true")]).expect_err("no");
        assert_eq!(refused, Refused::SecurityNotServed);
        assert_eq!(refused.status(), 0x0100);
    }
    let unknown = opens(&[("security", "plain"), ("ignore-cert", "true")]).expect_err("no");
    assert_eq!(
        unknown,
        Refused::BadValue {
            argument: "security"
        }
    );
}

#[test]
fn a_session_with_no_accepted_certificate_opens_only_when_the_operator_ignores_it() {
    let no_rule = opens(&[("security", "tls")]).expect_err("no rule for the certificate");
    assert_eq!(
        no_rule,
        Refused::BadValue {
            argument: "cert-fingerprints"
        }
    );
    // Only the exact text `true` is true, as for guacd.
    let not_true = opens(&[("ignore-cert", "yes")]).expect_err("not true");
    assert_eq!(
        not_true,
        Refused::BadValue {
            argument: "cert-fingerprints"
        }
    );

    assert_eq!(
        rdp(&[("ignore-cert", "true")]).certificate,
        Certificate::Any
    );
}

#[test]
fn an_accepted_certificate_is_checked_even_when_the_operator_also_ignores_certificates() {
    let both = rdp(&[("ignore-cert", "true"), ("cert-fingerprints", FINGERPRINT)]);

    assert!(matches!(both.certificate, Certificate::Accepted(_)));
}

#[test]
fn a_fingerprint_that_is_not_one_sha256_is_refused() {
    let short = FINGERPRINT.rsplit_once(':').expect("pairs").0;
    let long = format!("{FINGERPRINT}:20");
    let two = format!("{FINGERPRINT},{FINGERPRINT}");
    let other_hash = FINGERPRINT.replace("sha256", "sha1");
    let not_hex = FINGERPRINT.replace("0a", "zz");
    let signed = FINGERPRINT.replace("0a", "+a");
    for text in [
        short,
        &long,
        &two,
        &other_hash,
        &not_hex,
        &signed,
        "sha256",
        "sha256:",
    ] {
        let refused = opens(&[("cert-fingerprints", text)]).expect_err("refused");
        assert_eq!(
            refused,
            Refused::BadValue {
                argument: "cert-fingerprints"
            },
            "{text}"
        );
    }
}

#[test]
fn the_picture_is_scaled_unless_the_device_resizes_by_display_update() {
    let with = |method| rdp(&[("resize-method", method), ("ignore-cert", "true")]).resize;

    assert!(with("display-update"));
    assert!(!with("reconnect"));
    assert!(!with(""));
}

#[test]
fn a_domain_that_is_too_long_is_refused() {
    let domain = "d".repeat(256);
    let refused = opens(&[("domain", &domain), ("ignore-cert", "true")]).expect_err("refused");

    assert_eq!(refused, Refused::BadValue { argument: "domain" });
    assert_eq!(rdp(&[("ignore-cert", "true")]).domain, None);
}

#[test]
fn a_vnc_session_takes_no_rdp_value() {
    let (opening, _) = hub_opens("vnc", hub_values(&[("security", "tls")])).expect("opens");

    assert!(matches!(opening.desktop, Desktop::Vnc));
}
