//! `connect` values for RDP and VNC from the device, the password, the accepted certificate
//! and the drive root, answered to guacd 1.6.0's recorded `args` (K4 Section 3, GW-012,
//! GW-013, RA-021, RA-033, SEC-007).

mod switches;

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

use chrono::DateTime;
use guac_protocol::{ConnectParams, Decoder, Display, Handshake, HandshakeStep, Instruction};

use super::super::arguments::{BadConnectValue, Sources, audio, connect_values, time_zone};
use super::device;
use crate::fleet::credentials::Secret;
use crate::fleet::devices::Device;
use crate::fleet::{
    sessions::{Desktop, tests::Inputs},
    trust::ServerCertificate,
};

/// guacd 1.6.0's replies to `select,rdp` and `select,vnc`.
const RDP_ARGS: &[u8] = include_bytes!("fixtures/guacd-1.6.0-rdp-args.guac");
const VNC_ARGS: &[u8] = include_bytes!("fixtures/guacd-1.6.0-vnc-args.guac");

const DISPLAY: Display = Display {
    width: 1280,
    height: 800,
    dpi: 96,
};
const PASSWORD: &str = "correct horse battery staple";
/// The address the hub resolved `desk.example.net` to (R-032).
const TARGET: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 20, 0, 5));
const DRIVE_ROOT: &str = "/var/lib/guacd/drives";
const FINGERPRINT: &str = "0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9:\
                           0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9";

fn certificate() -> ServerCertificate {
    ServerCertificate {
        fingerprint_sha256: FINGERPRINT.to_owned(),
        subject: Some("CN=desk".to_owned()),
        not_after: None,
        first_seen_at: DateTime::UNIX_EPOCH,
    }
}

/// The `connect` values for `device` over `desktop` with the password and certificate.
fn values_for(desktop: Desktop, device: &Device) -> guac_protocol::ArgValues {
    let password = Secret::from(PASSWORD.to_owned());
    let certificate = certificate();
    let sources = Sources {
        device,
        target: TARGET,
        password: Some(&password),
        certificate: Some(&certificate),
        drive_root: DRIVE_ROOT,
    };
    connect_values(desktop, &sources).unwrap()
}

fn params(desktop: Desktop, device: &Device, zone: Option<&str>) -> ConnectParams {
    ConnectParams {
        display: DISPLAY,
        timezone: time_zone(zone),
        name: "alice".to_owned(),
        audio: audio(desktop, device),
        values: values_for(desktop, device),
    }
}

/// The fixture for `desktop`.
const fn fixture(desktop: Desktop) -> &'static [u8] {
    match desktop {
        Desktop::Rdp => RDP_ARGS,
        Desktop::Vnc => VNC_ARGS,
    }
}

/// What the handshake sends after the fixture's `args`, and the `args` names.
fn reply(desktop: Desktop, params: ConnectParams) -> (Vec<String>, Vec<Instruction>) {
    let args = Decoder::new().feed(fixture(desktop)).unwrap().remove(0);
    let names = args.args[1..].to_vec();
    let (mut handshake, select) = Handshake::new(desktop.select(), params);
    assert_eq!(select.args, [desktop.select()]);
    match handshake.on_instruction(args) {
        Ok(HandshakeStep::Send(sent)) => (names, sent),
        _ => panic!("the handshake answers args"),
    }
}

/// The `connect` value of each name: one value per name, each at its name's index.
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

/// The values the handshake sends for `device` over `desktop`.
fn connect_for(desktop: Desktop, device: &Device) -> HashMap<String, String> {
    let (names, sent) = reply(desktop, params(desktop, device, Some("Europe/Berlin")));
    connect_by_name(&names, &sent)
}

/// Every name has exactly its expected value, or empty; every expected name is asked for.
fn assert_exactly(values: &HashMap<String, String>, expected: &[(&str, &str)]) {
    for (name, value) in values {
        let want = expected
            .iter()
            .find(|(n, _)| n == name)
            .map_or("", |(_, v)| *v);
        assert_eq!(value, want, "value of {name}");
    }
    for (name, _) in expected {
        assert!(values.contains_key(*name), "{name} is in args");
    }
}

#[test]
fn k4_3_rdp_values_land_at_their_names_and_every_other_name_is_empty() {
    let device = device("desk.example.net");
    let fingerprint = format!("sha256:{FINGERPRINT}");
    let drive = format!("{DRIVE_ROOT}/{}", device.id);
    assert_exactly(
        &connect_for(Desktop::Rdp, &device),
        &[
            ("hostname", "10.20.0.5"),
            ("port", "3390"),
            ("username", "ops"),
            ("domain", "LAB"),
            ("password", PASSWORD),
            ("security", "nla-ext"),
            ("ignore-cert", "true"),
            ("cert-fingerprints", &fingerprint),
            ("resize-method", "reconnect"),
            ("enable-audio-input", "true"),
            ("enable-drive", "true"),
            ("drive-path", &drive),
            ("create-drive-path", "true"),
        ],
    );
}

#[test]
fn k4_3_vnc_values_land_at_their_names_and_every_other_name_is_empty() {
    let device = device("desk.example.net");
    assert_exactly(
        &connect_for(Desktop::Vnc, &device),
        &[
            ("hostname", "10.20.0.5"),
            ("port", "5901"),
            ("username", "viewer"),
            ("password", PASSWORD),
        ],
    );
}

#[test]
fn gw_013_paths_recordings_sftp_wake_and_trust_on_first_use_stay_empty() {
    let mut device = device("desk.example.net");
    device.rdp.shared_drive = false;
    for desktop in [Desktop::Rdp, Desktop::Vnc] {
        let values = connect_for(desktop, &device);
        for (name, value) in &values {
            let forbidden = ["recording-", "wol-", "sftp-", "typescript-"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
                || ["drive-path", "enable-sftp", "cert-tofu", "timezone"].contains(&name.as_str());
            if forbidden {
                assert_eq!(value, "", "{name} stays empty over {desktop:?}");
            }
        }
    }
}

#[test]
fn ra_021_each_device_gets_its_own_folder_under_the_drive_root() {
    let mut other = device("desk.example.net");
    other.id = "0190a000-0000-7000-8002-0000000000bb".parse().unwrap();
    for device in [device("desk.example.net"), other] {
        let values = connect_for(Desktop::Rdp, &device);
        assert_eq!(
            values["drive-path"],
            format!("/var/lib/guacd/drives/{}", device.id)
        );
    }
}

/// The `connect` values the handshake sends for `values` over `desktop`.
fn connect_by_name_of(
    desktop: Desktop,
    device: &Device,
    values: guac_protocol::ArgValues,
) -> HashMap<String, String> {
    let asked = ConnectParams {
        display: DISPLAY,
        timezone: time_zone(None),
        name: "alice".to_owned(),
        audio: audio(desktop, device),
        values,
    };
    let (names, sent) = reply(desktop, asked);
    connect_by_name(&names, &sent)
}

#[test]
fn ra_021_a_root_with_a_trailing_slash_gets_no_double_slash() {
    let device = device("desk");
    let password = Secret::from(PASSWORD.to_owned());
    let sources = Sources {
        device: &device,
        target: TARGET,
        password: Some(&password),
        certificate: None,
        drive_root: "/srv/drives/",
    };
    let values = connect_values(Desktop::Rdp, &sources).unwrap();
    let values = connect_by_name_of(Desktop::Rdp, &device, values);
    assert_eq!(values["drive-path"], format!("/srv/drives/{}", device.id));
    assert_eq!(values["cert-fingerprints"], "", "no certificate, no pin");
}

#[test]
fn k4_3_audio_lists_l8_and_l16_only_for_rdp_with_audio_out() {
    let mut device = device("desk");
    let sent_audio = |desktop: Desktop, device: &Device| {
        let (_, sent) = reply(desktop, params(desktop, device, None));
        sent.iter()
            .find(|i| i.opcode == "audio")
            .unwrap()
            .args
            .clone()
    };
    assert_eq!(sent_audio(Desktop::Rdp, &device), ["audio/L8", "audio/L16"]);
    assert_eq!(sent_audio(Desktop::Vnc, &device), Vec::<String>::new());
    device.rdp.audio_out = false;
    assert_eq!(sent_audio(Desktop::Rdp, &device), Vec::<String>::new());
}

#[test]
fn sec_007_r_032_guacd_gets_the_checked_address_never_the_host_name() {
    let device = device("desk.example.net");
    for desktop in [Desktop::Rdp, Desktop::Vnc] {
        let (names, sent) = reply(desktop, params(desktop, &device, None));
        assert_eq!(connect_by_name(&names, &sent)["hostname"], "10.20.0.5");
        let leaked = sent
            .iter()
            .any(|i| i.args.iter().any(|a| a.contains("desk.example")));
        assert!(!leaked, "the host name never reaches guacd");
    }
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
        "drive-path",
        "password",
        "8.hostname,9.evil.host;",
        "../../etc/passwd",
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
    let device = device("desk");
    let baseline = connect_for(Desktop::Rdp, &device);
    let mut inputs = Inputs(0x5eed_0604);
    let side = |inputs: &mut Inputs| 1 + u32::try_from(inputs.below(8192)).unwrap();
    for _ in 0..300 {
        let mut asked = params(Desktop::Rdp, &device, Some(&client_zone(&mut inputs)));
        asked.display = Display {
            width: side(&mut inputs),
            height: side(&mut inputs),
            dpi: 48 + u32::try_from(inputs.below(433)).unwrap(),
        };
        let zone = asked.timezone.clone();
        let (names, sent) = reply(Desktop::Rdp, asked);
        assert_eq!(connect_by_name(&names, &sent), baseline, "zone {zone:?}");
        assert_eq!(sent.len(), 7, "no extra instruction for zone {zone:?}");
    }
}

#[test]
fn log_007_a_password_that_is_not_text_is_refused_without_its_bytes() {
    let device = device("desk");
    let password = Secret::new(vec![0xff, 0xfe, b's', b'3', b'c']);
    let sources = Sources {
        device: &device,
        target: TARGET,
        password: Some(&password),
        certificate: None,
        drive_root: DRIVE_ROOT,
    };
    let error = connect_values(Desktop::Vnc, &sources).err().unwrap();
    assert_eq!(
        error,
        BadConnectValue::NotText {
            argument: "password"
        }
    );
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains("s3c") && !shown.contains("255"), "{shown}");
}
