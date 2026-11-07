//! guacd 1.6.0's recorded `args` for `rdp` and `vnc` hold every name K4 Section 3 sets.

use std::collections::HashSet;

use guac_protocol::Decoder;

/// guacd 1.6.0's reply to `select,rdp`, recorded from `guacamole/guacd:1.6.0`.
const RDP: &[u8] = include_bytes!("fixtures/guacd-1.6.0-rdp-args.guac");
/// guacd 1.6.0's reply to `select,vnc`, recorded from `guacamole/guacd:1.6.0`.
const VNC: &[u8] = include_bytes!("fixtures/guacd-1.6.0-vnc-args.guac");

/// The RDP names K4 Section 3 gives a value.
const RDP_NAMES: [&str; 18] = [
    "hostname",
    "port",
    "username",
    "domain",
    "password",
    "security",
    "ignore-cert",
    "cert-fingerprints",
    "resize-method",
    "disable-audio",
    "enable-audio-input",
    "enable-drive",
    "drive-path",
    "create-drive-path",
    "disable-download",
    "disable-upload",
    "disable-copy",
    "disable-paste",
];

/// The VNC names K4 Section 3 gives a value.
const VNC_NAMES: [&str; 6] = [
    "hostname",
    "port",
    "username",
    "password",
    "disable-copy",
    "disable-paste",
];

/// The Wake-on-LAN names both protocols ask for; K4 Section 3 always sends them empty.
const WOL_NAMES: [&str; 5] = [
    "wol-send-packet",
    "wol-mac-addr",
    "wol-broadcast-addr",
    "wol-udp-port",
    "wol-wait-time",
];

/// The names after the version in the fixture's one `args` instruction.
fn names(fixture: &[u8]) -> Vec<String> {
    let mut instructions = Decoder::new().feed(fixture).unwrap();
    assert_eq!(instructions.len(), 1, "one instruction per fixture");
    let args = instructions.remove(0);
    assert_eq!(args.opcode, "args");
    let (version, names) = args.args.split_first().unwrap();
    assert!(
        version.starts_with("VERSION_1_"),
        "version first: {version}"
    );
    names.to_vec()
}

fn missing<'a>(names: &[String], wanted: &[&'a str]) -> Vec<&'a str> {
    wanted
        .iter()
        .copied()
        .filter(|name| !names.iter().any(|have| have == name))
        .collect()
}

fn wol(names: &[String]) -> Vec<&str> {
    names
        .iter()
        .map(String::as_str)
        .filter(|name| name.starts_with("wol-"))
        .collect()
}

#[test]
fn rdp_args_hold_every_section_3_name() {
    let names = names(RDP);
    assert_eq!(names.len(), 88);
    assert_eq!(missing(&names, &RDP_NAMES), Vec::<&str>::new());
}

#[test]
fn vnc_args_hold_every_section_3_name() {
    let names = names(VNC);
    assert_eq!(names.len(), 51);
    assert_eq!(missing(&names, &VNC_NAMES), Vec::<&str>::new());
}

#[test]
fn rdp_args_offer_certificate_trust_on_first_use() {
    assert_eq!(missing(&names(RDP), &["cert-tofu"]), Vec::<&str>::new());
}

#[test]
fn vnc_args_move_files_only_over_sftp() {
    let names = names(VNC);
    assert_eq!(
        missing(&names, &["disable-download", "disable-upload"]),
        ["disable-download", "disable-upload"],
    );
    assert_eq!(missing(&names, &["enable-sftp"]), Vec::<&str>::new());
}

#[test]
fn both_protocols_ask_for_the_same_wake_on_lan_names() {
    assert_eq!(wol(&names(RDP)), WOL_NAMES);
    assert_eq!(wol(&names(VNC)), WOL_NAMES);
}

/// `connect` answers by position, so a repeated name would make one value ambiguous.
#[test]
fn no_name_repeats() {
    for fixture in [RDP, VNC] {
        let names = names(fixture);
        let unique: HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
    }
}
