//! The handshake fails closed: out-of-order instructions, gateway errors and
//! replies guacd would misread (GW-010, GW-013, LOG-007).

use guac_protocol::{
    ConnectParams, Display, Handshake, HandshakeError, HandshakeStep, Instruction, MAX_ELEMENTS,
};

use crate::guacd::{
    ARGS_1_5_5, ARGS_1_6_0, DISPLAY, K4_PASSWORD_VALUES, decode_args, params, values_of,
};

fn k4_params() -> ConnectParams {
    params(DISPLAY, "UTC", values_of(&K4_PASSWORD_VALUES))
}

fn started(params: ConnectParams) -> Handshake {
    Handshake::new("ssh", params).0
}

fn after_args() -> Handshake {
    let mut handshake = started(k4_params());
    handshake
        .on_instruction(decode_args(ARGS_1_5_5))
        .expect("args accepted");
    handshake
}

fn ready() -> Instruction {
    Instruction::new("ready", ["$4c1f6a0e"])
}

/// Every later instruction, even a valid one, returns `error` again.
fn assert_stays_failed(handshake: &mut Handshake, error: &HandshakeError) {
    for next in [
        decode_args(ARGS_1_5_5),
        ready(),
        Instruction::new("nop", Vec::<String>::new()),
    ] {
        assert_eq!(handshake.on_instruction(next).as_ref(), Err(error));
    }
}

#[test]
fn gw_010_gateway_error_before_args_or_ready_is_refused() {
    let error = Instruction::new("error", ["Upstream not found", "519"]);
    let expected = HandshakeError::Gateway {
        status: Some(519),
        message: "Upstream not found".to_owned(),
    };
    for mut handshake in [started(k4_params()), after_args()] {
        assert_eq!(
            handshake.on_instruction(error.clone()).as_ref(),
            Err(&expected)
        );
        assert_stays_failed(&mut handshake, &expected);
    }
}

#[test]
fn gw_010_gateway_error_without_numeric_status_keeps_the_message() {
    let mut handshake = started(k4_params());
    assert_eq!(
        handshake.on_instruction(Instruction::new("error", ["Aborted", "0x0207"])),
        Err(HandshakeError::Gateway {
            status: None,
            message: "Aborted".to_owned()
        })
    );
}

#[test]
fn gw_010_out_of_order_instructions_are_refused() {
    let cases = [
        (
            started(k4_params()),
            Instruction::new("sync", ["0"]),
            "args",
        ),
        (started(k4_params()), ready(), "args"),
        (after_args(), decode_args(ARGS_1_6_0), "ready"),
        (
            after_args(),
            Instruction::new("size", ["0", "1", "1"]),
            "ready",
        ),
    ];
    for (mut handshake, instruction, expected) in cases {
        let opcode = instruction.opcode.clone();
        let error = HandshakeError::UnexpectedInstruction { expected, opcode };
        assert_eq!(handshake.on_instruction(instruction).as_ref(), Err(&error));
        assert_stays_failed(&mut handshake, &error);
    }
}

#[test]
fn gw_010_any_instruction_after_ready_is_refused() {
    let mut handshake = after_args();
    assert!(matches!(
        handshake.on_instruction(ready()),
        Ok(HandshakeStep::Ready { .. })
    ));
    let nop = Instruction::new("nop", Vec::<String>::new());
    let error = handshake.on_instruction(nop).expect_err("after ready");
    assert!(matches!(
        error,
        HandshakeError::UnexpectedInstruction { ref opcode, .. } if opcode == "nop"
    ));
    assert_stays_failed(&mut handshake, &error);
}

#[test]
fn gw_010_unexpected_opcode_is_cut_to_32_characters() {
    let mut handshake = started(k4_params());
    let long = "x".repeat(100);
    let error = handshake
        .on_instruction(Instruction::new(long.clone(), Vec::<String>::new()))
        .expect_err("unexpected");
    let expected = format!("{}…", "x".repeat(32));
    assert!(matches!(
        &error,
        HandshakeError::UnexpectedInstruction { opcode, .. } if *opcode == expected
    ));
    assert!(!error.to_string().contains(&long));
}

#[test]
fn gw_013_display_out_of_bounds_is_refused_before_anything_is_sent() {
    let displays = [
        Display {
            width: 0,
            ..DISPLAY
        },
        Display {
            height: 8193,
            ..DISPLAY
        },
        Display { dpi: 47, ..DISPLAY },
        Display {
            dpi: 481,
            ..DISPLAY
        },
    ];
    for display in displays {
        let mut handshake = started(params(display, "UTC", values_of(&K4_PASSWORD_VALUES)));
        let error = HandshakeError::InvalidDisplay;
        assert_eq!(
            handshake.on_instruction(decode_args(ARGS_1_5_5)).as_ref(),
            Err(&error)
        );
        assert_stays_failed(&mut handshake, &error);
    }
}

#[test]
fn gw_013_nul_in_a_connect_value_is_refused_without_the_value() {
    let mut values = values_of(&K4_PASSWORD_VALUES);
    values.set("password", "hunter2\0hostname");
    let mut handshake = started(params(DISPLAY, "UTC", values));
    let error = handshake
        .on_instruction(decode_args(ARGS_1_5_5))
        .expect_err("NUL refused");
    assert_eq!(
        error,
        HandshakeError::NulInValue {
            opcode: "connect",
            argument: "password".to_owned()
        }
    );
    assert!(!error.to_string().contains("hunter2"));
    assert!(!format!("{error:?}").contains("hunter2"));
}

#[test]
fn gw_013_nul_in_timezone_or_name_is_refused() {
    let mut zoned = k4_params();
    zoned.timezone = "UTC\0Europe/Berlin".to_owned();
    let mut named = k4_params();
    named.name = "operator\0root".to_owned();
    for (params, opcode) in [(zoned, "timezone"), (named, "name")] {
        let mut handshake = started(params);
        assert_eq!(
            handshake.on_instruction(decode_args(ARGS_1_6_0)),
            Err(HandshakeError::NulInValue {
                opcode,
                argument: opcode.to_owned()
            })
        );
    }
}

#[test]
fn gw_013_oversize_private_key_is_refused_without_the_key() {
    let key = format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}",
        "QUJD".repeat(2048)
    );
    let mut values = values_of(&K4_PASSWORD_VALUES);
    values.set("private-key", key.clone());
    let mut handshake = started(params(DISPLAY, "UTC", values));
    let error = handshake
        .on_instruction(decode_args(ARGS_1_6_0))
        .expect_err("oversize refused");
    assert_eq!(
        error,
        HandshakeError::InstructionTooLarge {
            opcode: "connect".to_owned()
        }
    );
    assert!(!error.to_string().contains("QUJD"));
}

#[test]
fn log_007_debug_of_the_reply_omits_connect_values() {
    let mut handshake = started(k4_params());
    let step = handshake
        .on_instruction(decode_args(ARGS_1_5_5))
        .expect("args accepted");
    let debug = format!("{step:?}");
    assert!(debug.contains("connect"));
    assert!(!debug.contains("correct horse"));
    assert!(!debug.contains("10.20.0.15"));
}

#[test]
fn gw_013_connect_over_the_element_limit_is_refused() {
    let args_with = |names: usize| {
        let mut args = vec!["VERSION_1_5_0".to_owned()];
        args.extend((0..names).map(|n| format!("x-{n}")));
        Instruction::new("args", args)
    };
    let fits = started(k4_params()).on_instruction(args_with(MAX_ELEMENTS - 2));
    assert!(matches!(fits, Ok(HandshakeStep::Send(_))));
    assert_eq!(
        started(k4_params()).on_instruction(args_with(MAX_ELEMENTS - 1)),
        Err(HandshakeError::InstructionTooLarge {
            opcode: "connect".to_owned()
        })
    );
}
