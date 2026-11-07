//! The device's policy on guacd's streams (K4 Section 4, RA-021, RA-033): which the hub keeps
//! from the client, and which it answers with a refusing `ack`.

use guac_protocol::{Instruction, encode};

use super::super::relay::{ClientStep, GatewayStep, Relay};
use super::{decoded, relay_for};
use crate::devices::Device;
use crate::sessions::Desktop;

const LISTING: &str = "application/vnd.glyptodon.guacamole.stream-index+json";

fn rdp(edit: impl FnOnce(&mut Device)) -> Relay {
    relay_for(Desktop::Rdp, edit)
}

fn gateway(relay: &mut Relay, wire: &str) -> GatewayStep {
    relay.on_gateway(decoded(wire))
}

/// The `ack` that tells guacd the hub withholds download stream `stream`.
fn refused(stream: &str) -> GatewayStep {
    let text = "Download is turned off for this device.";
    GatewayStep::Refuse(Instruction::new("ack", [stream, text, "771"]))
}

#[test]
fn ra_033_copy_off_withholds_guacds_clipboard_stream_without_an_ack() {
    let mut relay = rdp(|device| device.transfer.copy = false);
    for wire in [
        "9.clipboard,1.5,10.text/plain;",
        "4.blob,1.5,8.c2VjcmV0;",
        "3.end,1.5;",
    ] {
        assert_eq!(gateway(&mut relay, wire), GatewayStep::Drop, "{wire}");
    }
    for wire in ["4.blob,1.5,4.aGk=;", "4.blob,1.6,4.aGk=;", "3.end,1.6;"] {
        assert_eq!(
            gateway(&mut relay, wire),
            GatewayStep::Forward(decoded(wire)),
            "{wire}"
        );
    }
}

#[test]
fn ra_033_while_a_stream_is_withheld_a_blob_with_a_garbled_index_is_dropped() {
    let mut relay = rdp(|device| device.transfer.copy = false);
    gateway(&mut relay, "9.clipboard,1.5,10.text/plain;");
    assert_eq!(
        gateway(&mut relay, "4.blob,2.5x,4.aGk=;"),
        GatewayStep::Drop
    );
    gateway(&mut relay, "3.end,1.5;");
    let wire = "4.blob,2.5x,4.aGk=;";
    assert_eq!(
        gateway(&mut relay, wire),
        GatewayStep::Forward(decoded(wire))
    );
}

#[test]
fn ra_033_copy_and_paste_on_relay_the_clipboard_both_ways() {
    let mut relay = rdp(|_| {});
    let pasted = decoded("9.clipboard,1.2,10.text/plain;");
    assert_eq!(relay.on_client(pasted.clone()), ClientStep::Forward(pasted));
    let wire = "9.clipboard,1.5,10.text/plain;";
    assert_eq!(
        gateway(&mut relay, wire),
        GatewayStep::Forward(decoded(wire))
    );
}

#[test]
fn ra_021_download_off_refuses_guacds_file_and_file_body_and_keeps_listings() {
    let mut relay = rdp(|device| device.transfer.download = false);
    assert_eq!(
        gateway(&mut relay, "4.file,1.7,10.text/plain,4.a.sh;"),
        refused("7")
    );
    assert_eq!(
        gateway(&mut relay, "4.blob,1.7,8.c2VjcmV0;"),
        GatewayStep::Drop
    );
    assert_eq!(gateway(&mut relay, "3.end,1.7;"), GatewayStep::Drop);
    let body = "4.body,1.0,1.8,10.text/plain,5./a.sh;";
    assert_eq!(gateway(&mut relay, body), refused("8"));
    let listing = encode_body("9", LISTING);
    assert_eq!(
        gateway(&mut relay, &listing),
        GatewayStep::Forward(decoded(&listing))
    );
}

#[test]
fn ra_021_without_the_drive_even_a_listing_is_refused() {
    let mut relay = rdp(|device| device.rdp.shared_drive = false);
    assert_eq!(
        gateway(&mut relay, &encode_body("9", LISTING)),
        refused("9")
    );
}

#[test]
fn ra_021_download_on_relays_guacds_file() {
    let mut relay = rdp(|_| {});
    let wire = "4.file,1.7,10.text/plain,4.a.sh;";
    assert_eq!(
        gateway(&mut relay, wire),
        GatewayStep::Forward(decoded(wire))
    );
}

#[test]
fn ra_021_vnc_never_sends_a_download_to_the_client() {
    let mut relay = relay_for(Desktop::Vnc, |_| {});
    assert_eq!(
        gateway(&mut relay, "4.file,1.7,10.text/plain,4.a.sh;"),
        refused("7")
    );
}

/// guacd's `body` for `get` of `/` on stream `stream` with `mimetype`.
fn encode_body(stream: &str, mimetype: &str) -> String {
    encode(&Instruction::new("body", ["0", stream, mimetype, "/"]))
}
