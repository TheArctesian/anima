//! The device's policy on client instructions (K4 Section 4, RA-021, RA-033, GW-015): what
//! each switch lets reach guacd, for RDP and VNC.

use guac_protocol::Instruction;
use proptest::prelude::*;

use super::super::ending::Cause;
use super::super::relay::{ClientStep, Relay};
use super::{decoded, relay_for};
use crate::devices::Device;
use crate::sessions::Desktop;

const CLOSE: ClientStep = ClientStep::End {
    forward: None,
    cause: Cause::InvalidInstruction,
};

fn rdp(edit: impl FnOnce(&mut Device)) -> Relay {
    relay_for(Desktop::Rdp, edit)
}

fn client(relay: &mut Relay, wire: &str) -> ClientStep {
    relay.on_client(decoded(wire))
}

fn forwards(relay: &mut Relay, wire: &str) {
    assert_eq!(
        client(relay, wire),
        ClientStep::Forward(decoded(wire)),
        "{wire}"
    );
}

#[test]
fn ra_033_paste_off_drops_the_clipboard_and_its_stream_and_the_session_stays_up() {
    let mut relay = rdp(|device| device.transfer.paste = false);
    for wire in [
        "9.clipboard,1.2,10.text/plain;",
        "4.blob,1.2,8.c2VjcmV0;",
        "3.end,1.2;",
    ] {
        assert_eq!(client(&mut relay, wire), ClientStep::Drop, "{wire}");
    }
    forwards(&mut relay, "3.key,5.65307,1.1;");
    assert_eq!(
        client(&mut relay, "4.blob,1.2,4.aGk=;"),
        CLOSE,
        "stream 2 ended"
    );
    assert_eq!(
        client(&mut relay, "4.blob,1.7,4.aGk=;"),
        CLOSE,
        "never opened"
    );
    assert_eq!(client(&mut relay, "9.clipboard,1.2;"), CLOSE, "too short");
}

#[test]
fn ra_033_paste_off_still_lets_a_credential_stream_through() {
    let mut relay = rdp(|device| device.transfer.paste = false);
    relay.on_gateway(decoded("8.required,8.password;"));
    assert_eq!(
        client(&mut relay, "9.clipboard,1.0,10.text/plain;"),
        ClientStep::Drop
    );
    forwards(&mut relay, "4.argv,1.0,10.text/plain,8.password;");
    forwards(&mut relay, "4.blob,1.0,4.aGk=;");
    forwards(&mut relay, "3.end,1.0;");
}

#[test]
fn ra_021_audio_input_needs_the_microphone() {
    let mut off = rdp(|device| device.rdp.microphone = false);
    assert_eq!(client(&mut off, "5.audio,1.1,9.audio/L16;"), CLOSE);
    let mut on = rdp(|_| {});
    forwards(&mut on, "5.audio,1.1,9.audio/L16;");
    forwards(&mut on, "4.blob,1.1,4.AAAA;");
    forwards(&mut on, "3.end,1.1;");
    assert_eq!(client(&mut on, "5.audio,1.1;"), CLOSE, "too short");
}

#[test]
fn ra_021_without_the_shared_drive_file_get_and_put_close() {
    let mut relay = rdp(|device| device.rdp.shared_drive = false);
    for wire in [
        "4.file,1.0,10.text/plain,4.a.sh;",
        "3.get,1.0,1./;",
        "3.put,1.0,1.1,10.text/plain,5./a.sh;",
    ] {
        assert_eq!(client(&mut relay, wire), CLOSE, "{wire}");
    }
}

#[test]
fn ra_021_with_the_drive_upload_off_refuses_file_and_put_and_allows_get() {
    let mut relay = rdp(|device| device.transfer.upload = false);
    for wire in [
        "4.file,1.0,10.text/plain,4.a.sh;",
        "3.put,1.0,1.1,10.text/plain,5./a.sh;",
    ] {
        assert_eq!(client(&mut relay, wire), CLOSE, "{wire}");
    }
    forwards(&mut relay, "3.get,1.0,1./;");
}

#[test]
fn ra_021_an_upload_opens_a_stream_for_blob_and_end() {
    let mut relay = rdp(|_| {});
    forwards(&mut relay, "4.file,1.3,10.text/plain,4.a.sh;");
    forwards(&mut relay, "4.blob,1.3,4.aGk=;");
    forwards(&mut relay, "3.end,1.3;");
    forwards(&mut relay, "3.put,1.0,1.4,10.text/plain,5./a.sh;");
    forwards(&mut relay, "4.blob,1.4,4.aGk=;");
    for wire in [
        "3.put,1.0,1.4,10.text/plain;",
        "3.put,2.+0,1.4,10.text/plain,5./a.sh;",
        "3.put,1.0,3.512,10.text/plain,5./a.sh;",
        "3.get,1.0;",
        "3.get,1.x,1./;",
        "4.file,1.3,10.text/plain;",
    ] {
        assert_eq!(client(&mut rdp(|_| {}), wire), CLOSE, "{wire}");
    }
}

#[test]
fn ra_021_vnc_never_takes_audio_input_or_the_drive() {
    let mut relay = relay_for(Desktop::Vnc, |_| {});
    for wire in [
        "5.audio,1.1,9.audio/L16;",
        "4.file,1.0,10.text/plain,4.a.sh;",
        "3.get,1.0,1./;",
        "3.put,1.0,1.1,10.text/plain,5./a.sh;",
    ] {
        assert_eq!(client(&mut relay, wire), CLOSE, "{wire}");
    }
}

#[test]
fn gw_015_rdp_answers_username_password_and_domain_only() {
    let mut relay = rdp(|_| {});
    relay.on_gateway(decoded(
        "8.required,8.username,8.password,6.domain,10.passphrase;",
    ));
    for (stream, name) in [("0", "username"), ("1", "password"), ("2", "domain")] {
        let argv = Instruction::new("argv", [stream, "text/plain", name]);
        assert_eq!(relay.on_client(argv.clone()), ClientStep::Forward(argv));
    }
    let passphrase = Instruction::new("argv", ["3", "text/plain", "passphrase"]);
    assert_eq!(relay.on_client(passphrase), CLOSE);
}

#[test]
fn gw_015_vnc_answers_username_and_password_only() {
    let mut relay = relay_for(Desktop::Vnc, |_| {});
    relay.on_gateway(decoded("8.required,8.username,8.password,6.domain;"));
    for (stream, name) in [("0", "username"), ("1", "password")] {
        let argv = Instruction::new("argv", [stream, "text/plain", name]);
        assert_eq!(relay.on_client(argv.clone()), ClientStep::Forward(argv));
    }
    let domain = Instruction::new("argv", ["2", "text/plain", "domain"]);
    assert_eq!(relay.on_client(domain), CLOSE);
}

/// The opcodes a policy with every switch off must never let reach guacd.
const REFUSED: [&str; 5] = ["clipboard", "audio", "file", "get", "put"];

fn opcode() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "clipboard",
        "audio",
        "file",
        "get",
        "put",
        "blob",
        "end",
        "argv",
        "key",
        "ack",
    ])
}

fn argument() -> impl Strategy<Value = String> {
    prop_oneof![
        (0_u16..4).prop_map(|n| n.to_string()),
        Just("text/plain".to_owned()),
        Just("password".to_owned()),
        "[0-9a-z+ /.-]{0,6}",
    ]
}

fn instruction() -> impl Strategy<Value = Instruction> {
    (opcode(), prop::collection::vec(argument(), 0..6))
        .prop_map(|(opcode, args)| Instruction::new(opcode, args))
}

/// The stream index a forwarded instruction names in `args[0]`.
fn index(sent: &Instruction) -> u32 {
    sent.args[0]
        .parse()
        .expect("forwarded stream indices are numbers")
}

/// Every client stream a forwarded instruction opened and has not ended.
fn opens(sent: &Instruction, open: &mut Vec<u32>) {
    match sent.opcode.as_str() {
        "argv" => open.push(index(sent)),
        "end" => open.retain(|stream| *stream != index(sent)),
        _ => {}
    }
}

proptest! {
    #[test]
    fn ra_033_no_client_sequence_gets_a_refused_opcode_or_its_stream_to_guacd(
        vnc in any::<bool>(),
        sequence in prop::collection::vec(instruction(), 0..40),
    ) {
        let desktop = if vnc { Desktop::Vnc } else { Desktop::Rdp };
        let mut relay = relay_for(desktop, |device| {
            device.rdp.microphone = false;
            device.rdp.shared_drive = false;
            device.transfer.paste = false;
            device.transfer.upload = false;
        });
        relay.on_gateway(decoded("8.required,8.password;"));
        let mut open = Vec::new();
        for instruction in sequence {
            let sent = match relay.on_client(instruction) {
                ClientStep::Forward(sent) => sent,
                ClientStep::End { .. } => break,
                ClientStep::Drop | ClientStep::Reply(_) => continue,
            };
            prop_assert!(!REFUSED.contains(&sent.opcode.as_str()), "{sent:?}");
            if matches!(sent.opcode.as_str(), "blob" | "end") {
                prop_assert!(open.contains(&index(&sent)), "{sent:?} on {open:?}");
            }
            opens(&sent, &mut open);
        }
    }
}
