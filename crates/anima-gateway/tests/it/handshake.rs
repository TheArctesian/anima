//! The gateway answers the hub's handshake as guacd does, and refuses what it cannot use.

use anima_gateway::handshake::{Handshake, Opening, Refused, Secret, Step, error, ready};
use guac_protocol::{
    ArgValues, ConnectParams, Display, Handshake as HubHandshake, HandshakeStep, Instruction,
};
use proptest::prelude::*;

const PASSWORD: &str = "correct horse battery";

fn hub_values(extra: &[(&str, &str)]) -> ArgValues {
    let mut values = ArgValues::new();
    values.set("hostname", "192.0.2.7");
    values.set("port", "5900");
    for (name, value) in extra {
        values.set(*name, *value);
    }
    values
}

/// Runs the hub's own handshake against the gateway's, as `tunnel/guacd.rs` does.
fn hub_opens(protocol: &str, values: ArgValues) -> Result<(Opening, String), Refused> {
    let params = ConnectParams {
        display: Display {
            width: 1440,
            height: 900,
            dpi: 96,
        },
        timezone: "Europe/Berlin".to_owned(),
        name: "operator".to_owned(),
        audio: &[],
        values,
    };
    let (mut hub, select) = HubHandshake::new(protocol, params);
    let mut gateway = Handshake::default();
    let Step::Send(args) = gateway.on_instruction(select)? else {
        panic!("args after select")
    };
    let HandshakeStep::Send(sent) = hub.on_instruction(args).expect("the hub takes args") else {
        panic!("the hub answers args")
    };
    let mut opening = None;
    for instruction in sent {
        if let Step::Opened(opened) = gateway.on_instruction(instruction)? {
            opening = Some(*opened);
        }
    }
    let HandshakeStep::Ready { connection_id } = hub
        .on_instruction(ready("$4f0c"))
        .expect("the hub takes ready")
    else {
        panic!("the hub is ready")
    };
    Ok((opening.expect("connect opens the session"), connection_id))
}

fn refused(values: &[(&str, &str)]) -> Refused {
    hub_opens("vnc", hub_values(values)).expect_err("refused")
}

#[test]
fn the_hubs_handshake_opens_a_vnc_session() {
    let values = hub_values(&[
        ("username", "ada"),
        ("password", PASSWORD),
        ("disable-paste", "true"),
    ]);
    let (opening, connection_id) = hub_opens("vnc", values).expect("opened");
    assert_eq!(connection_id, "$4f0c");
    assert_eq!(
        opening.device,
        "192.0.2.7:5900".parse().expect("an address")
    );
    assert_eq!(opening.username.as_ref().map(Secret::expose), Some("ada"));
    assert_eq!(
        opening.password.as_ref().map(Secret::expose),
        Some(PASSWORD)
    );
    assert_eq!((opening.copy, opening.paste), (true, false));
    assert_eq!(opening.view, (1440, 900));
    assert!(opening.formats.jpeg);
}

#[test]
fn a_session_without_credentials_opens_with_none() {
    let (opening, _) = hub_opens("vnc", hub_values(&[])).expect("opened");
    assert!(opening.username.is_none() && opening.password.is_none());
    assert_eq!((opening.copy, opening.paste), (true, true));
}

#[test]
fn nothing_printed_about_an_opening_holds_the_password() {
    let values = hub_values(&[("username", "ada"), ("password", PASSWORD)]);
    let (opening, _) = hub_opens("vnc", values).expect("opened");
    let printed = format!("{opening:?}");
    assert!(
        !printed.contains(PASSWORD) && !printed.contains("ada"),
        "{printed}"
    );
}

#[test]
fn a_protocol_the_gateway_does_not_serve_is_refused_as_unsupported() {
    let refusal = hub_opens("rdp", hub_values(&[])).expect_err("refused");
    assert_eq!(refusal, Refused::NotServed);
    assert_eq!(error(refusal.status()).args, ["Aborted. See logs.", "256"]);
}

#[test]
fn a_device_given_by_name_is_refused_so_the_gateway_never_resolves() {
    let refusal = refused(&[("hostname", "device.example")]);
    assert_eq!(
        refusal,
        Refused::BadValue {
            argument: "hostname"
        }
    );
    assert!(!refusal.to_string().contains("device.example"));
    assert_eq!(refusal.status(), 0x0300);
}

#[test]
fn an_address_that_is_never_a_device_is_refused() {
    for address in [
        "0.0.0.0",
        "::",
        "224.0.0.1",
        "255.255.255.255",
        "ff02::1",
        "::ffff:192.0.2.7",
    ] {
        assert_eq!(
            refused(&[("hostname", address)]),
            Refused::BadValue {
                argument: "hostname"
            },
            "{address}"
        );
    }
}

#[test]
fn a_client_that_draws_png_only_gets_no_jpeg() {
    let mut gateway = Handshake::default();
    gateway
        .on_instruction(Instruction::new("select", ["vnc"]))
        .expect("args");
    gateway
        .on_instruction(Instruction::new("size", ["800", "600", "96"]))
        .expect("size");
    gateway
        .on_instruction(Instruction::new("image", ["image/png"]))
        .expect("image");
    let connect = Instruction::new(
        "connect",
        ["VERSION_1_5_0", "192.0.2.7", "5900", "", "", "", ""],
    );
    let Ok(Step::Opened(opening)) = gateway.on_instruction(connect) else {
        panic!("opened")
    };
    assert!(!opening.formats.jpeg);
}

#[test]
fn a_port_that_is_not_a_port_is_refused() {
    for port in ["0", "65536", "", "5900 ", "-1"] {
        assert_eq!(
            refused(&[("port", port)]),
            Refused::BadValue { argument: "port" },
            "{port:?}"
        );
    }
}

#[test]
fn a_password_that_is_too_long_is_refused_without_being_shown() {
    let long = "p".repeat(4097);
    let refusal = refused(&[("password", &long)]);
    assert_eq!(
        refusal,
        Refused::BadValue {
            argument: "password"
        }
    );
    assert!(!format!("{refusal} {refusal:?}").contains("ppp"));
}

#[test]
fn instructions_out_of_order_are_refused() {
    let connect = || {
        Instruction::new(
            "connect",
            ["VERSION_1_5_0", "192.0.2.7", "5900", "", "", "", ""],
        )
    };
    let select = || Instruction::new("select", ["vnc"]);
    assert_eq!(
        Handshake::default().on_instruction(connect()).err(),
        Some(Refused::NotSelect)
    );

    let mut no_size = Handshake::default();
    no_size.on_instruction(select()).expect("args");
    assert_eq!(
        no_size.on_instruction(connect()).err(),
        Some(Refused::NoSize)
    );
    assert_eq!(
        no_size.on_instruction(select()).err(),
        Some(Refused::Unexpected)
    );

    let mut twice = Handshake::default();
    twice.on_instruction(select()).expect("args");
    assert_eq!(
        twice.on_instruction(select()).err(),
        Some(Refused::Unexpected)
    );
}

#[test]
fn a_connect_with_the_wrong_number_of_values_is_refused() {
    let mut gateway = Handshake::default();
    gateway
        .on_instruction(Instruction::new("select", ["vnc"]))
        .expect("args");
    gateway
        .on_instruction(Instruction::new("size", ["800", "600", "96"]))
        .expect("size");
    let short = Instruction::new("connect", ["VERSION_1_5_0", "192.0.2.7", "5900"]);
    assert_eq!(
        gateway.on_instruction(short).err(),
        Some(Refused::BadValue {
            argument: "connect"
        })
    );
}

#[test]
fn a_view_outside_the_tunnel_limits_is_refused() {
    for size in [["0", "600"], ["800", "8193"], ["wide", "600"]] {
        let mut gateway = Handshake::default();
        gateway
            .on_instruction(Instruction::new("select", ["vnc"]))
            .expect("args");
        let refusal = gateway.on_instruction(Instruction::new("size", size)).err();
        assert_eq!(refusal, Some(Refused::BadValue { argument: "size" }));
    }
}

proptest! {
    /// Whatever the peer sends, the handshake answers or refuses; it does not panic.
    #[test]
    fn no_instructions_break_the_handshake(
        instructions in prop::collection::vec(
            (
                prop::sample::select(vec!["select", "size", "image", "audio", "connect", "nop", "key", ""]),
                prop::collection::vec("[a-zA-Z0-9_./:-]{0,12}", 0..9),
            ),
            0..12,
        ),
    ) {
        let mut gateway = Handshake::default();
        for (opcode, args) in instructions {
            let _ = gateway.on_instruction(Instruction::new(opcode, args));
        }
    }
}
