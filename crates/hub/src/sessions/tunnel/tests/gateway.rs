//! What guacd's instructions after `ready` mean (K4 Sections 4 and 5, SEC-007, GW-015).

use guac_protocol::{Instruction, encode};

use super::super::ending::EndCause;
use super::super::gateway::host_key_changed;
use super::super::relay::{GatewayStep, Relay};
use super::decoded;

fn step(wire: &str) -> GatewayStep {
    Relay::new().on_gateway(decoded(wire))
}

fn ends(wire: &str, cause: EndCause) {
    let send = decoded(wire);
    assert_eq!(step(wire), GatewayStep::End { send, cause }, "{wire}");
}

#[test]
fn k4_display_instructions_are_forwarded_unchanged() {
    for wire in [
        "4.sync,13.1730700000000,1.1;",
        "3.img,1.0,2.14,1.0,9.image/png,1.0,1.0;",
        "4.blob,1.0,4.aGk=;",
        "3.nop;",
        "3.ack,1.0,2.OK,1.0;",
        "4.argv,1.0,10.text/plain,12.color-scheme;",
    ] {
        assert_eq!(step(wire), GatewayStep::Forward(decoded(wire)), "{wire}");
    }
}

#[test]
fn gw_015_required_is_forwarded_and_its_credentials_noted() {
    let mut relay = Relay::new();
    let required = decoded("8.required,8.username,8.password,8.hostname;");
    assert_eq!(
        relay.on_gateway(required.clone()),
        GatewayStep::Forward(required)
    );
    assert_eq!(
        relay.prompted.iter().copied().collect::<Vec<_>>(),
        ["password", "username"]
    );
}

#[test]
fn k4_gateway_disconnect_is_relayed_then_ends_remote_closed() {
    ends("10.disconnect;", EndCause::RemoteClosed);
}

#[test]
fn k4_error_unauthorized_or_forbidden_is_relayed_then_ends_auth_failed() {
    ends("5.error,18.Aborted. See logs.,3.769;", EndCause::AuthFailed);
    ends("5.error,18.Aborted. See logs.,3.771;", EndCause::AuthFailed);
}

#[test]
fn k4_any_other_error_is_relayed_then_ends_gateway_error() {
    for wire in [
        "5.error,18.Aborted. See logs.,3.512;",
        "5.error,18.Aborted. See logs.,3.515;",
        "5.error,18.Aborted. See logs.,3.768;",
        "5.error,18.Aborted. See logs.;",
        "5.error;",
        "5.error,18.Aborted. See logs.,4.+769;",
        "5.error,18.Aborted. See logs.,6.0x0301;",
        "5.error,18.Aborted. See logs.,4. 769;",
        "5.error,18.Aborted. See logs.,6.000769;",
    ] {
        ends(wire, EndCause::GatewayError);
    }
}

/// guacd 1.5.5 and 1.6.0 send a host-key mismatch as `guac_client_abort`'s fixed text with
/// status 0x0200 (`src/common-ssh/ssh.c` and `src/libguac/client.c`, cited in `gateway.rs`).
/// The words "Host key did not match any provided known host keys." only reach guacd's log,
/// so the wire form is a plain gateway error, and the hub never trusts error text for it.
#[test]
fn sec_007_guacds_host_key_abort_is_a_gateway_error_on_the_wire() {
    ends(
        "5.error,18.Aborted. See logs.,3.512;",
        EndCause::GatewayError,
    );
    ends(
        "5.error,53.Host key did not match any provided known host keys. ,3.512;",
        EndCause::GatewayError,
    );
}

#[test]
fn sec_007_the_hub_sends_host_key_changed_with_status_0x0303() {
    assert_eq!(
        encode(&host_key_changed()),
        "5.error,16.Host key changed,3.771;"
    );
}

#[test]
fn gw_014_internal_instructions_from_guacd_are_dropped() {
    assert_eq!(step("0.,4.ping,1.1;"), GatewayStep::Drop);
    assert_eq!(
        step("0.,36.00000000-0000-0000-0000-000000000000;"),
        GatewayStep::Drop
    );
}

#[test]
fn gw_015_required_names_stay_bounded() {
    let mut relay = Relay::new();
    for _ in 0..100 {
        relay.on_gateway(Instruction::new("required", ["password", "domain", "x"]));
    }
    assert_eq!(relay.prompted.len(), 2);
}
