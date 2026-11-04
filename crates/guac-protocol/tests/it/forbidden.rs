//! Client input never reaches a `connect` value or adds an instruction (GW-013).
//!
//! The client controls only `Display` and the time zone (GW-012). Whatever
//! they hold, every `connect` value is the hub's value for that name (or `""`),
//! and the encoded reply decodes back to exactly the instructions sent.

use std::collections::HashMap;

use guac_protocol::{ArgValues, Decoder, Display, Instruction, encode};
use proptest::prelude::*;

use crate::guacd::{ARGS_1_5_5, ARGS_1_6_0, connect_of, decode_args, params, reply};

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

/// Time zones shaped like argument names, assignments and wire instructions.
fn timezone() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("hostname".to_owned()),
        Just("port=22".to_owned()),
        Just(";".to_owned()),
        Just("4.port,2.22;".to_owned()),
        Just("8.hostname,9.evil.host;7.connect;".to_owned()),
        Just("Europe/Zürich 東京 😀".to_owned()),
        text(64),
    ]
}

fn fixture() -> impl Strategy<Value = Instruction> {
    prop_oneof![Just(decode_args(ARGS_1_5_5)), Just(decode_args(ARGS_1_6_0))]
}

/// Hub values: names from the fixtures or arbitrary, short values. Later pairs win.
fn hub_pairs() -> impl Strategy<Value = Vec<(String, String)>> {
    let names: Vec<String> = decode_args(ARGS_1_6_0).args[1..].to_vec();
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
}
