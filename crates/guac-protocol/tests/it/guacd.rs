//! guacd `args` fixtures and the hub values of K4 Section 3.

use guac_protocol::{
    ArgValues, ConnectParams, Decoder, Display, Handshake, HandshakeStep, Instruction,
};

/// `args` of the official guacamole/guacd image (no `--enable-ssh-agent`, GW-001).
pub const ARGS_1_5_5: &[u8] = include_bytes!("fixtures/guacd-1.5.5-ssh-args.guac");
pub const ARGS_1_6_0: &[u8] = include_bytes!("fixtures/guacd-1.6.0-ssh-args.guac");
/// `args` of guacd built with `--enable-ssh-agent` (adds `enable-agent`).
pub const AGENT_ARGS_1_5_5: &[u8] = include_bytes!("fixtures/guacd-1.5.5-ssh-agent-args.guac");
pub const AGENT_ARGS_1_6_0: &[u8] = include_bytes!("fixtures/guacd-1.6.0-ssh-agent-args.guac");

pub const DISPLAY: Display = Display {
    width: 1024,
    height: 768,
    dpi: 96,
};

/// K4 Section 3 Phase 1 SSH values, password variant (synthetic credentials).
pub const K4_PASSWORD_VALUES: [(&str, &str); 11] = [
    ("hostname", "10.20.0.15"),
    ("port", "22"),
    ("username", "anima"),
    ("password", "correct horse battery staple"),
    (
        "host-key",
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKexampleexampleexample",
    ),
    (
        "color-scheme",
        "foreground:rgb:d8/de/e9;background:rgb:2e/34/40",
    ),
    ("font-name", "OpenDyslexicM Nerd Font Mono"),
    ("font-size", "12"),
    ("scrollback", "5000"),
    ("terminal-type", "xterm-256color"),
    ("server-alive-interval", "30"),
];

/// The `args` instruction in `bytes`, decoded with the crate's own decoder.
pub fn decode_args(bytes: &[u8]) -> Instruction {
    let mut decoded = Decoder::new().feed(bytes).expect("fixture decodes");
    assert_eq!(decoded.len(), 1, "a fixture holds one instruction");
    decoded.remove(0)
}

/// Every fixture with its number of names after the version.
pub fn all_fixtures() -> [(Instruction, usize); 4] {
    [
        (decode_args(ARGS_1_5_5), 38),
        (decode_args(ARGS_1_6_0), 42),
        (decode_args(AGENT_ARGS_1_5_5), 39),
        (decode_args(AGENT_ARGS_1_6_0), 43),
    ]
}

pub fn values_of(pairs: &[(&str, &str)]) -> ArgValues {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

pub fn params(display: Display, timezone: &str, values: ArgValues) -> ConnectParams {
    ConnectParams {
        display,
        timezone: timezone.to_owned(),
        name: "operator".to_owned(),
        values,
    }
}

/// Runs `select` then `args` and returns what the hub sends to guacd.
pub fn reply(args: Instruction, params: ConnectParams) -> Vec<Instruction> {
    let (mut handshake, _select) = Handshake::new("ssh", params);
    match handshake.on_instruction(args) {
        Ok(HandshakeStep::Send(send)) => send,
        Ok(HandshakeStep::Ready { .. }) => panic!("args does not end the handshake"),
        Err(error) => panic!("handshake refused: {error}"),
    }
}

/// The `connect` instruction of a reply (always the last one).
pub fn connect_of(reply: &[Instruction]) -> &Instruction {
    let connect = reply.last().expect("reply is not empty");
    assert_eq!(connect.opcode, "connect");
    connect
}

/// The `connect` value sent for `name`, given the `args` it answers.
pub fn value_for<'a>(args: &Instruction, connect: &'a Instruction, name: &str) -> &'a str {
    let at = args
        .args
        .iter()
        .position(|n| n == name)
        .expect("name in args");
    &connect.args[at]
}
