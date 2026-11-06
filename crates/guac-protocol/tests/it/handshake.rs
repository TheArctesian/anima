//! The handshake reply to guacd `args` fixtures (GW-010, GW-014).

use guac_protocol::{
    ArgValues, Handshake, HandshakeError, HandshakeStep, Instruction, MAX_INSTRUCTION_BYTES, encode,
};

use crate::guacd::{
    ARGS_1_5_5, ARGS_1_6_0, DISPLAY, K4_PASSWORD_VALUES, after_args, all_fixtures, connect_of,
    decode_args, k4_params, params, reply, started, value_for, values_of,
};

const ONLY_IN_1_6: [&str; 4] = [
    "timeout",
    "public-key",
    "typescript-write-existing",
    "recording-write-existing",
];

fn k4_reply(args: &Instruction) -> Vec<Instruction> {
    reply(
        args.clone(),
        params(DISPLAY, "Europe/Berlin", values_of(&K4_PASSWORD_VALUES)),
    )
}

#[test]
fn gw_010_fixtures_decode_to_versioned_ssh_args() {
    for (args, names) in all_fixtures() {
        assert_eq!(args.opcode, "args");
        assert_eq!(args.args[0], "VERSION_1_5_0");
        assert_eq!(args.args.len(), names + 1);
    }
    let [old, new, old_agent, new_agent] = all_fixtures().map(|(args, _)| args);
    for name in ONLY_IN_1_6 {
        assert!(!old.args.iter().any(|n| n == name));
        assert!(new.args.iter().any(|n| n == name));
    }
    let has_agent = |args: &Instruction| args.args.iter().any(|n| n == "enable-agent");
    assert!(!has_agent(&old) && !has_agent(&new));
    assert!(has_agent(&old_agent) && has_agent(&new_agent));
}

#[test]
fn gw_010_select_names_the_protocol() {
    let (_, select) = Handshake::new("ssh", params(DISPLAY, "UTC", ArgValues::new()));
    assert_eq!(encode(&select), "6.select,3.ssh;");
}

#[test]
fn gw_010_reply_sends_display_media_timezone_and_name_before_connect() {
    let send = k4_reply(&decode_args(ARGS_1_5_5));
    let encoded: Vec<String> = send.iter().map(encode).collect();
    assert_eq!(
        encoded[..6],
        [
            "4.size,4.1024,3.768,2.96;",
            "5.audio;",
            "5.video;",
            "5.image,9.image/png,10.image/jpeg,10.image/webp;",
            "8.timezone,13.Europe/Berlin;",
            "4.name,8.operator;",
        ]
    );
    assert_eq!(send.len(), 7);
    assert_eq!(connect_of(&send).opcode, "connect");
}

#[test]
fn gw_010_connect_has_version_then_one_value_per_name_in_order() {
    for (args, _) in all_fixtures() {
        let send = k4_reply(&args);
        let connect = connect_of(&send);
        assert_eq!(connect.args.len(), args.args.len());
        assert_eq!(connect.args[0], "VERSION_1_5_0");
        for (name, value) in args.args[1..].iter().zip(&connect.args[1..]) {
            let expected = K4_PASSWORD_VALUES
                .iter()
                .find(|(n, _)| n == name)
                .map_or("", |(_, v)| v);
            assert_eq!(value, expected, "value for {name}");
        }
    }
}

/// `(position in connect, value)` for the K4 values in the official-image fixtures.
const POSITIONS_1_5_5: [(usize, &str); 12] = [
    (1, "10.20.0.15"),
    (
        2,
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKexampleexampleexample",
    ),
    (3, "22"),
    (4, "anima"),
    (5, "correct horse battery staple"),
    (6, "OpenDyslexicM Nerd Font Mono"),
    (7, "12"),
    (12, ""),
    (14, "foreground:rgb:d8/de/e9;background:rgb:2e/34/40"),
    (26, "30"),
    (28, "xterm-256color"),
    (29, "5000"),
];
const POSITIONS_1_6_0: [(usize, &str); 12] = [
    (1, "10.20.0.15"),
    (3, "22"),
    (4, ""),
    (5, "anima"),
    (6, "correct horse battery staple"),
    (7, "OpenDyslexicM Nerd Font Mono"),
    (8, "12"),
    (15, ""),
    (16, "foreground:rgb:d8/de/e9;background:rgb:2e/34/40"),
    (30, "30"),
    (32, "xterm-256color"),
    (33, "5000"),
];

#[test]
fn gw_010_connect_places_k4_values_at_their_fixture_positions() {
    for (fixture, positions) in [(ARGS_1_5_5, POSITIONS_1_5_5), (ARGS_1_6_0, POSITIONS_1_6_0)] {
        let send = k4_reply(&decode_args(fixture));
        let connect = &connect_of(&send).args;
        for (at, value) in positions {
            assert_eq!(connect[at], value, "connect value {at}");
        }
    }
}

#[test]
fn gw_010_connect_sends_private_key_and_passphrase() {
    let args = decode_args(ARGS_1_6_0);
    let key = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjE=\n-----END OPENSSH PRIVATE KEY-----\n";
    let values = values_of(&[("private-key", key), ("passphrase", "pass phrase")]);
    let send = reply(args.clone(), params(DISPLAY, "UTC", values));
    let connect = connect_of(&send);
    assert_eq!(value_for(&args, connect, "private-key"), key);
    assert_eq!(value_for(&args, connect, "passphrase"), "pass phrase");
    assert_eq!(value_for(&args, connect, "password"), "");
}

#[test]
fn gw_010_names_only_in_1_6_are_empty_unless_set() {
    let args = decode_args(ARGS_1_6_0);
    let send = k4_reply(&args);
    for name in ONLY_IN_1_6 {
        assert_eq!(value_for(&args, connect_of(&send), name), "");
    }
    let mut values = values_of(&K4_PASSWORD_VALUES);
    values.set("timeout", "10");
    let send = reply(args.clone(), params(DISPLAY, "UTC", values));
    assert_eq!(value_for(&args, connect_of(&send), "timeout"), "10");
}

#[test]
fn gw_010_answers_version_1_5_0_to_a_later_version() {
    let mut args = decode_args(ARGS_1_6_0);
    args.args[0] = "VERSION_1_6_0".to_owned();
    let send = k4_reply(&args);
    assert_eq!(connect_of(&send).args[0], "VERSION_1_5_0");
}

#[test]
fn gw_010_refuses_old_or_unparseable_versions() {
    let refused = [
        "VERSION_1_3_0",
        "VERSION_1_4_9",
        "NOT_A_VERSION",
        "VERSION_1_5",
        "VERSION_1_5_0_0",
        "VERSION_1_+5_0",
        "VERSION_1__5_0",
        "version_1_5_0",
        "1_5_0",
        "XVERSION_1_5_0",
        "",
    ];
    for version in refused {
        let mut args = decode_args(ARGS_1_5_5);
        args.args[0] = version.to_owned();
        assert_eq!(
            started(k4_params())
                .on_instruction(args)
                .expect_err("refused"),
            HandshakeError::UnsupportedVersion,
            "{version:?}"
        );
    }
    let empty = Instruction::new("args", Vec::<String>::new());
    assert_eq!(
        started(k4_params())
            .on_instruction(empty)
            .expect_err("refused"),
        HandshakeError::UnsupportedVersion
    );
}

#[test]
fn gw_014_ready_returns_the_gateway_connection_id() {
    let mut handshake = after_args();
    let ready = Instruction::new("ready", ["$4c1f6a0e-7f3b-4e2a-9c55-1d2e3f4a5b6c"]);
    assert_eq!(
        handshake.on_instruction(ready),
        Ok(HandshakeStep::Ready {
            connection_id: "$4c1f6a0e-7f3b-4e2a-9c55-1d2e3f4a5b6c".to_owned()
        })
    );
}

#[test]
fn gw_014_ready_without_connection_id_is_refused() {
    for ready in [
        Instruction::new("ready", Vec::<String>::new()),
        Instruction::new("ready", [""]),
    ] {
        let mut handshake = after_args();
        assert_eq!(
            handshake.on_instruction(ready),
            Err(HandshakeError::MissingConnectionId)
        );
    }
}

#[test]
fn gw_010_nop_is_ignored_before_ready() {
    let nop = || Instruction::new("nop", Vec::<String>::new());
    let (mut handshake, _) = Handshake::new("ssh", params(DISPLAY, "UTC", ArgValues::new()));
    assert_eq!(
        handshake.on_instruction(nop()),
        Ok(HandshakeStep::Send(vec![]))
    );
    handshake
        .on_instruction(decode_args(ARGS_1_5_5))
        .expect("args accepted");
    assert_eq!(
        handshake.on_instruction(nop()),
        Ok(HandshakeStep::Send(vec![]))
    );
    assert!(matches!(
        handshake.on_instruction(Instruction::new("ready", ["$id"])),
        Ok(HandshakeStep::Ready { .. })
    ));
}

#[test]
fn gw_010_connect_at_the_byte_limit_is_sent_and_one_byte_more_is_refused() {
    let args = decode_args(ARGS_1_5_5);
    let base = encode(connect_of(&k4_reply(&args))).len();
    // An unset `private-key` encodes as `0.`; a key of n ASCII chars as `<n>.<key>`.
    let key_chars = MAX_INSTRUCTION_BYTES - base - 4 + 1;
    let with_key = |chars: usize| {
        let mut values = values_of(&K4_PASSWORD_VALUES);
        values.set("private-key", "k".repeat(chars));
        params(DISPLAY, "UTC", values)
    };
    let send = reply(args.clone(), with_key(key_chars));
    assert_eq!(encode(connect_of(&send)).len(), MAX_INSTRUCTION_BYTES);

    let (mut handshake, _) = Handshake::new("ssh", with_key(key_chars + 1));
    assert_eq!(
        handshake.on_instruction(args),
        Err(HandshakeError::InstructionTooLarge { opcode: "connect" })
    );
}
