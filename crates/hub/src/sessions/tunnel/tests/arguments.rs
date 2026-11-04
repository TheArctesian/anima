//! `connect` values from the device, the secret and the host key (K4 Section 3, GW-012,
//! GW-013, RA-010, RA-011, SEC-006).

use std::collections::HashMap;

use guac_protocol::{ConnectParams, Decoder, Display, Handshake, HandshakeStep, Instruction};

use super::super::arguments::{BadConnectValue, connect_values, time_zone};
use super::super::known_hosts::{BadHost, known_hosts_line};
use super::{device, host_key};
use crate::credentials::{Secret, SshSecret};
use crate::sessions::tests::{ED25519, Inputs};

/// guacd's SSH `args`, built from guacamole-server's `GUAC_SSH_CLIENT_ARGS` (WP-08 fixtures).
const FIXTURES: [&[u8]; 4] = [
    include_bytes!("../../../../../guac-protocol/tests/it/fixtures/guacd-1.5.5-ssh-args.guac"),
    include_bytes!("../../../../../guac-protocol/tests/it/fixtures/guacd-1.6.0-ssh-args.guac"),
    include_bytes!(
        "../../../../../guac-protocol/tests/it/fixtures/guacd-1.5.5-ssh-agent-args.guac"
    ),
    include_bytes!(
        "../../../../../guac-protocol/tests/it/fixtures/guacd-1.6.0-ssh-agent-args.guac"
    ),
];

const DISPLAY: Display = Display {
    width: 1280,
    height: 800,
    dpi: 96,
};

const PASSWORD: &str = "correct horse battery staple";
const PRIVATE_KEY: &str =
    "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----\n";
const PASSPHRASE: &str = "open sesame";

fn password() -> SshSecret {
    SshSecret::Password(Secret::from(PASSWORD.to_owned()))
}

fn private_key() -> SshSecret {
    SshSecret::PrivateKey {
        key: Secret::from(PRIVATE_KEY.to_owned()),
        passphrase: Some(Secret::from(PASSPHRASE.to_owned())),
    }
}

fn params(secret: Option<&SshSecret>, display: Display, zone: Option<&str>) -> ConnectParams {
    ConnectParams {
        display,
        timezone: time_zone(zone),
        name: "alice".to_owned(),
        values: connect_values(&device("db.example.net", 2222), secret, &host_key()).unwrap(),
    }
}

/// The instructions the handshake sends after the fixture's `args`, and the `args` names.
fn reply(fixture: &[u8], params: ConnectParams) -> (Vec<String>, Vec<Instruction>) {
    let args = Decoder::new().feed(fixture).unwrap().remove(0);
    let names = args.args[1..].to_vec();
    let (mut handshake, _select) = Handshake::new("ssh", params);
    match handshake.on_instruction(args) {
        Ok(HandshakeStep::Send(sent)) => (names, sent),
        _ => panic!("the handshake answers args"),
    }
}

/// The `connect` value of each name, checking each sits at its name's position.
fn connect_by_name(names: &[String], sent: &[Instruction]) -> HashMap<String, String> {
    let connect = sent.last().unwrap();
    assert_eq!(connect.opcode, "connect");
    assert_eq!(connect.args[0], "VERSION_1_5_0");
    assert_eq!(connect.args.len(), names.len() + 1, "one value per name");
    names
        .iter()
        .cloned()
        .zip(connect.args[1..].iter().cloned())
        .collect()
}

/// The K4 Section 3 value of every non-empty name for the password device.
fn k4_values(secret: &[(&str, &str)]) -> HashMap<String, String> {
    let line = format!("[db.example.net]:2222 ssh-ed25519 {ED25519}");
    let scheme = anima_theme::guac_color_scheme();
    let fixed = [
        ("hostname", "db.example.net"),
        ("port", "2222"),
        ("username", "ops"),
        ("host-key", line.as_str()),
        ("color-scheme", scheme.as_str()),
        ("font-name", "OpenDyslexicM Nerd Font Mono"),
        ("font-size", "12"),
        ("scrollback", "5000"),
        ("terminal-type", "xterm-256color"),
        ("server-alive-interval", "30"),
    ];
    fixed
        .iter()
        .chain(secret)
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// Every name the fixture lists has exactly its K4 value, or empty.
fn assert_k4(fixture: &[u8], secret: Option<&SshSecret>, expected: &HashMap<String, String>) {
    let (names, sent) = reply(fixture, params(secret, DISPLAY, Some("Europe/Berlin")));
    let values = connect_by_name(&names, &sent);
    for name in &names {
        let want = expected.get(name).map_or("", String::as_str);
        assert_eq!(values[name], want, "value of {name}");
    }
    for name in [
        "enable-sftp",
        "recording-path",
        "typescript-path",
        "timezone",
    ] {
        assert_eq!(values[name], "", "{name} stays empty");
    }
    assert!(
        expected.keys().all(|name| names.contains(name)),
        "every K4 name is in args"
    );
}

#[test]
fn gw_012_k4_password_values_fill_every_guacd_args_list() {
    let expected = k4_values(&[("password", PASSWORD)]);
    for fixture in FIXTURES {
        assert_k4(fixture, Some(&password()), &expected);
    }
}

#[test]
fn gw_012_k4_private_key_values_fill_every_guacd_args_list() {
    let expected = k4_values(&[("private-key", PRIVATE_KEY), ("passphrase", PASSPHRASE)]);
    for fixture in FIXTURES {
        assert_k4(fixture, Some(&private_key()), &expected);
    }
}

#[test]
fn gw_015_no_stored_secret_sends_no_credential() {
    for fixture in FIXTURES {
        assert_k4(fixture, None, &k4_values(&[]));
    }
}

#[test]
fn ra_010_ra_011_color_scheme_and_font_come_from_the_hub() {
    let (names, sent) = reply(FIXTURES[1], params(None, DISPLAY, None));
    let values = connect_by_name(&names, &sent);
    assert_eq!(values["color-scheme"], anima_theme::guac_color_scheme());
    assert_eq!(values["font-name"], "OpenDyslexicM Nerd Font Mono");
    assert_eq!(values["font-size"], "12");
}

#[test]
fn k4_the_handshake_sends_the_display_zone_and_operator_name() {
    let (_, sent) = reply(FIXTURES[0], params(None, DISPLAY, Some("America/New_York")));
    let by_opcode = |opcode: &str| {
        sent.iter()
            .find(|i| i.opcode == opcode)
            .unwrap()
            .args
            .clone()
    };
    assert_eq!(by_opcode("size"), ["1280", "800", "96"]);
    assert_eq!(by_opcode("timezone"), ["America/New_York"]);
    assert_eq!(by_opcode("name"), ["alice"]);
}

#[test]
fn k4_time_zone_is_the_clients_when_zone_shaped_else_utc() {
    assert_eq!(time_zone(Some("Europe/Berlin")), "Europe/Berlin");
    assert_eq!(time_zone(Some("Etc/GMT+5")), "Etc/GMT+5");
    for bad in [
        "",
        "/etc/passwd",
        ":Europe/Berlin",
        "UTC; rm -rf",
        "a//b",
        "4.port,2.22;",
    ] {
        assert_eq!(time_zone(Some(bad)), "UTC", "{bad:?}");
    }
    assert_eq!(time_zone(None), "UTC");
}

/// Hostile and random zone text, including argument names and wire syntax.
fn client_zone(inputs: &mut Inputs) -> String {
    const HOSTILE: [&str; 8] = [
        "hostname",
        "port",
        "password",
        "8.hostname,9.evil.host;",
        "private-key/password",
        "Etc/GMT+5\0",
        "/proc/self/environ",
        "Europe/Zürich",
    ];
    const ALPHABET: &[u8] = b"Aa0_+-/.,;: \n\0";
    if inputs.below(3) == 0 {
        HOSTILE[inputs.below(HOSTILE.len())].to_owned()
    } else {
        inputs.text(ALPHABET, 70)
    }
}

#[test]
fn gw_013_no_client_display_or_zone_changes_a_connect_value() {
    let (names, baseline) = reply(FIXTURES[1], params(Some(&password()), DISPLAY, None));
    let baseline = connect_by_name(&names, &baseline);
    let mut inputs = Inputs(0x5eed_0604);
    let side = |inputs: &mut Inputs| 1 + u32::try_from(inputs.below(8192)).unwrap();
    for _ in 0..300 {
        let display = Display {
            width: side(&mut inputs),
            height: side(&mut inputs),
            dpi: 48 + u32::try_from(inputs.below(433)).unwrap(),
        };
        let zone = client_zone(&mut inputs);
        let (names, sent) = reply(FIXTURES[1], params(Some(&password()), display, Some(&zone)));
        assert_eq!(connect_by_name(&names, &sent), baseline, "zone {zone:?}");
        assert_eq!(sent.len(), 7, "no extra instruction for zone {zone:?}");
    }
}

#[test]
fn sec_006_host_key_line_has_no_brackets_on_port_22() {
    let key = host_key().public_key;
    let line = known_hosts_line("db.example.net", 22, &key).unwrap();
    assert_eq!(line, format!("db.example.net ssh-ed25519 {ED25519}"));
    let line = known_hosts_line("10.20.0.15", 22, &key).unwrap();
    assert_eq!(line, format!("10.20.0.15 ssh-ed25519 {ED25519}"));
}

#[test]
fn sec_006_host_key_line_brackets_the_host_on_other_ports() {
    let key = host_key().public_key;
    let line = known_hosts_line("db.example.net", 2222, &key).unwrap();
    assert_eq!(line, format!("[db.example.net]:2222 ssh-ed25519 {ED25519}"));
    let line = known_hosts_line("fd7a:115c::1", 2200, &key).unwrap();
    assert_eq!(line, format!("[fd7a:115c::1]:2200 ssh-ed25519 {ED25519}"));
}

#[test]
fn sec_006_host_key_line_refuses_a_host_that_would_change_the_line() {
    let key = host_key().public_key;
    let long = "a".repeat(255);
    let bad = [
        "",
        "db host",
        "db,evil",
        "|1|salt|hash",
        "#db",
        "@revoked",
        "[db]",
        "db\n",
        "db\t",
        "db\0",
        &long,
    ];
    for host in bad {
        assert_eq!(known_hosts_line(host, 22, &key), Err(BadHost), "{host:?}");
    }
}

#[test]
fn sec_006_host_key_line_host_field_is_at_most_254_bytes() {
    let key = host_key().public_key;
    assert!(known_hosts_line(&"a".repeat(254), 22, &key).is_ok());
    assert_eq!(known_hosts_line(&"a".repeat(255), 22, &key), Err(BadHost));
    // `[` + 247 + `]:2222` is 254 bytes; one more is 255.
    assert!(known_hosts_line(&"a".repeat(247), 2222, &key).is_ok());
    assert_eq!(known_hosts_line(&"a".repeat(248), 2222, &key), Err(BadHost));
}

#[test]
fn sec_006_host_key_line_keeps_an_underscore_host() {
    let key = host_key().public_key;
    let line = known_hosts_line("db_1.example", 22, &key).unwrap();
    assert_eq!(line, format!("db_1.example ssh-ed25519 {ED25519}"));
}

#[test]
fn sec_006_a_host_that_cannot_be_pinned_refuses_the_connect_values() {
    let error = connect_values(&device("db host", 22), None, &host_key())
        .err()
        .unwrap();
    assert_eq!(error, BadConnectValue::Host);
}

#[test]
fn log_007_a_secret_that_is_not_text_is_refused_without_its_bytes() {
    let secret = SshSecret::Password(Secret::new(vec![0xff, 0xfe, b's', b'3', b'c']));
    let error = connect_values(&device("db", 22), Some(&secret), &host_key())
        .err()
        .unwrap();
    assert_eq!(
        error,
        BadConnectValue::NotText {
            argument: "password"
        }
    );
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains("s3c") && !shown.contains("255"), "{shown}");
}
