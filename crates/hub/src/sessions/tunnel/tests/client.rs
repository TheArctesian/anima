//! Which client instructions reach guacd (GW-011, GW-012, GW-013, GW-015, K4 Section 4).

use guac_protocol::{Instruction, encode};

use super::super::ending::Cause;
use super::super::relay::{ClientStep, Relay};
use super::{decoded, prompted};
use crate::sessions::tests::Inputs;

const CLOSE: ClientStep = ClientStep::End {
    forward: None,
    cause: Cause::InvalidInstruction,
};

fn step(relay: &mut Relay, wire: &str) -> ClientStep {
    relay.on_client(decoded(wire))
}

fn forwards(relay: &mut Relay, wire: &str) {
    assert_eq!(
        step(relay, wire),
        ClientStep::Forward(decoded(wire)),
        "{wire}"
    );
}

#[test]
fn gw_011_ping_is_answered_with_itself_and_not_forwarded() {
    let ping = decoded("0.,4.ping,13.1730700000000;");
    assert_eq!(
        Relay::default().on_client(ping.clone()),
        ClientStep::Reply(ping)
    );
}

#[test]
fn gw_011_other_internal_instructions_are_dropped() {
    for wire in [
        "0.;",
        "0.,4.PING;",
        "0.,36.00000000-0000-0000-0000-000000000000;",
    ] {
        assert_eq!(
            step(&mut Relay::default(), wire),
            ClientStep::Drop,
            "{wire}"
        );
    }
}

#[test]
fn gw_011_input_instructions_are_forwarded_unchanged() {
    let mut relay = Relay::default();
    for wire in [
        "3.key,5.65307,1.1;",
        "5.mouse,3.100,2.50,1.1;",
        "5.touch,1.0,3.100,2.50,1.5,1.5,1.0,1.1;",
        "4.sync,13.1730700000000;",
        "3.nop;",
        "3.ack,1.4,2.OK,1.0;",
    ] {
        forwards(&mut relay, wire);
    }
}

#[test]
fn gw_011_client_disconnect_is_forwarded_then_ends_as_the_operator() {
    let disconnect = decoded("10.disconnect;");
    assert_eq!(
        Relay::default().on_client(disconnect.clone()),
        ClientStep::End {
            forward: Some(disconnect),
            cause: Cause::OperatorClosed,
        }
    );
}

#[test]
fn gw_011_an_opcode_outside_the_allow_list_closes_with_1002() {
    for wire in [
        "4.file,1.0,10.text/plain,4.a.sh;",
        "4.pipe,1.0,10.text/plain,1.p;",
        "3.get,1.0,1./;",
        "3.put,1.0,1.0,10.text/plain,1./;",
        "5.audio,1.0,9.audio/L16;",
        "6.select,3.ssh;",
        "7.connect,13.VERSION_1_5_0;",
        "3.img,1.0,2.14,1.0,9.image/png,1.0,1.0;",
        "3.KEY,1.1,1.1;",
    ] {
        assert_eq!(step(&mut Relay::default(), wire), CLOSE, "{wire}");
    }
}

#[test]
fn gw_011_an_instruction_shorter_than_guacd_reads_closes() {
    for wire in [
        "3.key,5.65307;",
        "5.mouse,3.100,2.50;",
        "5.touch,1.0,3.100,2.50,1.5,1.5,1.0;",
        "4.sync;",
        "3.ack,1.4,2.OK;",
        "9.clipboard,1.0;",
        "4.argv,1.0,10.text/plain;",
    ] {
        assert_eq!(step(&mut Relay::default(), wire), CLOSE, "{wire}");
    }
}

#[test]
fn gw_011_an_ack_with_a_non_numeric_stream_or_status_closes() {
    for wire in [
        "3.ack,2.+4,2.OK,1.0;",
        "3.ack,1.4,2.OK,2.-1;",
        "3.ack,0.,2.OK,1.0;",
        "3.ack,6.000004,2.OK,1.0;",
        "3.ack,1.4,2.OK,6.000000;",
    ] {
        assert_eq!(step(&mut Relay::default(), wire), CLOSE, "{wire}");
    }
}

#[test]
fn gw_013_argv_for_a_hub_value_closes() {
    for name in [
        "hostname",
        "port",
        "host-key",
        "recording-path",
        "font-name",
        "wol-mac-addr",
    ] {
        let wire = encode(&Instruction::new("argv", ["0", "text/plain", name]));
        assert_eq!(step(&mut Relay::default(), &wire), CLOSE, "{name}");
    }
}

#[test]
fn gw_015_argv_for_a_credential_guacd_did_not_ask_for_closes() {
    assert_eq!(
        step(
            &mut Relay::default(),
            "4.argv,1.0,10.text/plain,8.password;"
        ),
        CLOSE
    );
    let mut relay = prompted(&["username"]);
    assert_eq!(
        step(&mut relay, "4.argv,1.0,10.text/plain,8.password;"),
        CLOSE
    );
}

#[test]
fn gw_015_required_then_argv_opens_a_stream_for_blob_and_end() {
    let mut relay = prompted(&["password"]);
    forwards(&mut relay, "4.argv,1.3,10.text/plain,8.password;");
    forwards(&mut relay, "4.blob,1.3,8.aHVudGVy;");
    forwards(&mut relay, "3.end,1.3;");
    assert_eq!(
        step(&mut relay, "4.blob,1.3,8.aHVudGVy;"),
        CLOSE,
        "stream 3 ended"
    );
}

#[test]
fn gw_015_a_prompt_is_answered_once() {
    let mut relay = prompted(&["password"]);
    forwards(&mut relay, "4.argv,1.0,10.text/plain,8.password;");
    assert_eq!(
        step(&mut relay, "4.argv,1.1,10.text/plain,8.password;"),
        CLOSE
    );
}

#[test]
fn gw_015_required_for_a_hub_value_does_not_make_it_answerable() {
    let mut relay = prompted(&["hostname", "port"]);
    assert_eq!(
        step(&mut relay, "4.argv,1.0,10.text/plain,8.hostname;"),
        CLOSE
    );
}

#[test]
fn gw_012_the_client_may_set_the_colour_scheme() {
    let mut relay = Relay::default();
    forwards(&mut relay, "4.argv,1.0,10.text/plain,12.color-scheme;");
    forwards(&mut relay, "4.blob,1.0,8.Z3JheQ==;");
    forwards(&mut relay, "3.end,1.0;");
}

#[test]
fn gw_011_clipboard_opens_a_client_stream() {
    let mut relay = Relay::default();
    forwards(&mut relay, "9.clipboard,1.2,10.text/plain;");
    forwards(&mut relay, "4.blob,1.2,4.aGk=;");
    forwards(&mut relay, "3.end,1.2;");
}

#[test]
fn gw_011_blob_or_end_on_a_stream_the_client_did_not_open_closes() {
    let mut relay = Relay::default();
    forwards(&mut relay, "9.clipboard,1.2,10.text/plain;");
    for wire in [
        "4.blob,1.5,4.aGk=;",
        "3.end,1.5;",
        "4.blob,2.+2,4.aGk=;",
        "3.end,3.02 ;",
    ] {
        assert_eq!(step(&mut relay, wire), CLOSE, "{wire}");
    }
}

#[test]
fn gw_011_a_stream_index_past_guacds_limit_closes() {
    assert_eq!(
        step(&mut Relay::default(), "9.clipboard,3.512,10.text/plain;"),
        CLOSE
    );
    assert_eq!(
        step(&mut Relay::default(), "9.clipboard,2.-1,10.text/plain;"),
        CLOSE
    );
    assert_eq!(
        step(&mut Relay::default(), "9.clipboard,6.000002,10.text/plain;"),
        CLOSE,
        "six digits"
    );
    forwards(&mut Relay::default(), "9.clipboard,5.00511,10.text/plain;");
    forwards(&mut Relay::default(), "3.ack,5.00004,2.OK,5.00000;");
}

/// Random instructions over the allow-listed opcodes and some others.
fn random_instruction(inputs: &mut Inputs) -> Instruction {
    const OPCODES: [&str; 16] = [
        "",
        "size",
        "key",
        "mouse",
        "touch",
        "sync",
        "nop",
        "disconnect",
        "ack",
        "clipboard",
        "argv",
        "blob",
        "end",
        "file",
        "connect",
        "required",
    ];
    const ALPHABET: &[u8] = b"0123456789+- .pingtextpasswordcolor-scheme";
    let opcode = OPCODES[inputs.below(OPCODES.len())];
    let args: Vec<String> = (0..inputs.below(8))
        .map(|_| inputs.text(ALPHABET, 6))
        .collect();
    Instruction::new(opcode, args)
}

/// The fewest arguments guacd's handler reads for each forwarded opcode.
fn arity(opcode: &str) -> Option<usize> {
    let table = [
        ("size", 2),
        ("key", 2),
        ("mouse", 3),
        ("touch", 7),
        ("sync", 1),
        ("nop", 0),
        ("ack", 3),
        ("clipboard", 2),
        ("argv", 3),
        ("blob", 2),
        ("end", 1),
        ("disconnect", 0),
    ];
    table
        .iter()
        .find(|(name, _)| *name == opcode)
        .map(|(_, n)| *n)
}

#[test]
fn gw_011_only_allowed_opcodes_with_their_arguments_reach_guacd() {
    let mut inputs = Inputs(0x5eed_0611);
    let mut relay = prompted(&["password"]);
    for _ in 0..20_000 {
        let sent = match relay.on_client(random_instruction(&mut inputs)) {
            ClientStep::Forward(i)
            | ClientStep::End {
                forward: Some(i), ..
            } => i,
            ClientStep::Reply(i) => {
                assert_eq!(i.opcode, "");
                continue;
            }
            ClientStep::Drop | ClientStep::End { forward: None, .. } => continue,
        };
        let needed = arity(&sent.opcode).expect("an allowed opcode");
        assert!(sent.args.len() >= needed, "{sent:?}");
    }
}

#[test]
fn log_007_a_step_debug_shows_no_argument() {
    let mut relay = prompted(&["password"]);
    relay.on_client(decoded("4.argv,1.0,10.text/plain,8.password;"));
    let step = step(&mut relay, "4.blob,1.0,12.aHVudGVyMg==;");
    let shown = format!("{step:?}");
    assert!(!shown.contains("aHVudGVyMg"), "{shown}");
}
