//! Client input never reaches a `connect` value or adds an instruction (GW-013).
//!
//! The client controls only `Display` and the time zone (GW-012). Whatever
//! they hold, every `connect` value is the hub's value for that name (or `""`),
//! and the encoded reply decodes back to exactly the instructions sent. A time
//! zone outside the zone-name charset is refused before anything is sent.

use std::collections::HashMap;

use guac_protocol::{ArgValues, Decoder, Display, Handshake, HandshakeError, Instruction, encode};
use proptest::prelude::*;

use crate::guacd::{AGENT_ARGS_1_6_0, all_fixtures, connect_of, decode_args, params, reply};

/// Names the client must never set, plus every `wol-*` name (GW-013).
const FORBIDDEN: [&str; 11] = [
    "hostname",
    "port",
    "username",
    "password",
    "private-key",
    "passphrase",
    "public-key",
    "recording-path",
    "drive-path",
    "sftp-root-directory",
    "typescript-path",
];

fn is_forbidden(name: &str) -> bool {
    FORBIDDEN.contains(&name) || name.starts_with("wol-")
}

/// Any Unicode text but NUL (the handshake refuses NUL values by design).
fn text(max_chars: usize) -> impl Strategy<Value = String> {
    let non_nul = any::<char>().prop_filter("no NUL", |c| *c != '\0');
    prop::collection::vec(non_nul, 0..=max_chars).prop_map(String::from_iter)
}

fn display() -> impl Strategy<Value = Display> {
    let side = Display::MIN_SIDE_PX..=Display::MAX_SIDE_PX;
    (side.clone(), side, Display::MIN_DPI..=Display::MAX_DPI)
        .prop_map(|(width, height, dpi)| Display { width, height, dpi })
}

/// Valid zone names, including ones spelled like argument names (GW-012).
fn timezone() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("hostname".to_owned()),
        Just("port".to_owned()),
        Just("wol-send-packet".to_owned()),
        Just("private-key/password".to_owned()),
        Just("Etc/GMT+5".to_owned()),
        Just("Etc/GMT-14".to_owned()),
        // A letter, then up to three non-empty `/` segments: at most 63 characters.
        "[A-Za-z][A-Za-z0-9_+-]{0,20}(/[A-Za-z0-9_+-]{1,20}){0,2}",
    ]
}

/// True for a character a zone name may hold.
fn zone_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '/')
}

/// Time zones that are not zone names: hostile shapes, zone characters in a
/// non-IANA shape, more than 64 characters, or any character outside the charset.
fn bad_timezone() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("port=22".to_owned()),
        Just(";".to_owned()),
        Just("4.port,2.22;".to_owned()),
        Just("8.hostname,9.evil.host;7.connect;".to_owned()),
        Just("Europe/Zürich 東京 😀".to_owned()),
        "[A-Za-z0-9_+/-]{65,80}",
        // Zone characters in a non-IANA shape: leading `/` (a glibc file path),
        // leading non-letter, empty segment, trailing `/`.
        "/[A-Za-z0-9_+/-]{0,20}",
        "[0-9_+-][A-Za-z0-9_+/-]{0,20}",
        "[A-Za-z][A-Za-z0-9_+-]{0,10}//[A-Za-z0-9_+/-]{0,10}",
        "[A-Za-z][A-Za-z0-9_+/-]{0,20}/",
        text(64).prop_filter("has a non-zone character", |t| !t.chars().all(zone_char)),
    ]
}

fn fixture() -> impl Strategy<Value = Instruction> {
    prop::sample::select(all_fixtures().map(|(args, _)| args).to_vec())
}

/// Hub values: names from the fixtures or arbitrary, short values. Later pairs win.
fn hub_pairs() -> impl Strategy<Value = Vec<(String, String)>> {
    let names: Vec<String> = decode_args(AGENT_ARGS_1_6_0).args[1..].to_vec();
    let name = prop_oneof![prop::sample::select(names), text(16)];
    prop::collection::vec((name, text(32)), 0..48)
}

fn hub_values(pairs: &[(String, String)]) -> ArgValues {
    pairs.iter().cloned().collect::<ArgValues>()
}

fn expected_values(pairs: &[(String, String)]) -> HashMap<&str, &str> {
    pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

proptest! {
    #[test]
    fn gw_013_connect_values_come_only_from_the_hub(
        args in fixture(),
        display in display(),
        timezone in timezone(),
        pairs in hub_pairs(),
    ) {
        let send = reply(args.clone(), params(display, &timezone, hub_values(&pairs)));
        let connect = connect_of(&send);
        let hub = expected_values(&pairs);
        prop_assert_eq!(connect.args.len(), args.args.len());
        prop_assert_eq!(connect.args[0].as_str(), "VERSION_1_5_0");
        let mut forbidden_seen = 0;
        for (name, value) in args.args[1..].iter().zip(&connect.args[1..]) {
            let expected = hub.get(name.as_str()).copied().unwrap_or("");
            prop_assert_eq!(value.as_str(), expected, "value for {}", name);
            forbidden_seen += usize::from(is_forbidden(name));
        }
        // hostname, port, username, password, private-key, passphrase,
        // recording-path, sftp-root-directory, typescript-path and five wol-*.
        prop_assert!(forbidden_seen >= 14);
    }

    #[test]
    fn gw_013_encoded_reply_decodes_to_the_same_instructions(
        args in fixture(),
        display in display(),
        timezone in timezone(),
        pairs in hub_pairs(),
    ) {
        let send = reply(args, params(display, &timezone, hub_values(&pairs)));
        let wire: String = send.iter().map(encode).collect();
        let decoded = Decoder::new().feed(wire.as_bytes());
        prop_assert_eq!(decoded, Ok(send.clone()));
        prop_assert_eq!(&send[4].args, &vec![timezone]);
    }

    #[test]
    fn gw_013_client_inputs_never_change_connect(
        args in fixture(),
        first in (display(), timezone()),
        second in (display(), timezone()),
        pairs in hub_pairs(),
    ) {
        let one = reply(args.clone(), params(first.0, &first.1, hub_values(&pairs)));
        let two = reply(args, params(second.0, &second.1, hub_values(&pairs)));
        prop_assert_eq!(connect_of(&one), connect_of(&two));
    }

    #[test]
    fn gw_012_time_zone_outside_the_charset_is_refused(
        args in fixture(),
        display in display(),
        timezone in bad_timezone(),
        pairs in hub_pairs(),
    ) {
        let (mut handshake, _) = Handshake::new("ssh", params(display, &timezone, hub_values(&pairs)));
        prop_assert_eq!(handshake.on_instruction(args), Err(HandshakeError::InvalidTimezone));
    }
}
