//! What guacd's instructions after `ready` mean (K4 Sections 4 and 5, SEC-007, GW-015, D-11).

use guac_protocol::Instruction;

use super::super::ending::Cause;
use super::super::relay::{ClientStep, GatewayStep};
use super::{decoded, relay};

fn step(wire: &str) -> GatewayStep {
    relay().on_gateway(decoded(wire))
}

/// guacd's `error` in `wire` reaches the client as the fixed text with `status`, then ends.
fn ends_with_error(wire: &str, status: &str, cause: Cause) {
    let send = Instruction::new("error", ["Aborted. See logs.", status]);
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
        "8.required,8.password;",
    ] {
        assert_eq!(step(wire), GatewayStep::Forward(decoded(wire)), "{wire}");
    }
}

#[test]
fn gw_015_required_makes_only_its_credentials_answerable() {
    let mut relay = relay();
    relay.on_gateway(decoded("8.required,8.username,8.hostname;"));
    let argv = |name: &str| Instruction::new("argv", ["0", "text/plain", name]);
    assert_eq!(
        relay.on_client(argv("username")),
        ClientStep::Forward(argv("username"))
    );
    for name in ["hostname", "password"] {
        assert!(
            matches!(
                relay.on_client(argv(name)),
                ClientStep::End { forward: None, .. }
            ),
            "{name}"
        );
    }
}

#[test]
fn k4_gateway_disconnect_is_relayed_then_ends_remote_closed() {
    let send = decoded("10.disconnect;");
    assert_eq!(
        step("10.disconnect;"),
        GatewayStep::End {
            send,
            cause: Cause::RemoteClosed
        }
    );
}

#[test]
fn k4_error_unauthorized_or_forbidden_is_relayed_then_ends_auth_failed() {
    ends_with_error(
        "5.error,18.Aborted. See logs.,3.769;",
        "769",
        Cause::AuthFailed,
    );
    ends_with_error(
        "5.error,18.Aborted. See logs.,3.771;",
        "771",
        Cause::AuthFailed,
    );
}

#[test]
fn k4_any_other_error_is_relayed_then_ends_gateway_error() {
    for (wire, status) in [
        ("5.error,18.Aborted. See logs.,3.512;", "512"),
        ("5.error,18.Aborted. See logs.,3.515;", "515"),
        ("5.error,18.Aborted. See logs.,3.768;", "768"),
        ("5.error,18.Aborted. See logs.,5.01023;", "1023"),
    ] {
        ends_with_error(wire, status, Cause::GatewayError);
    }
}

#[test]
fn k4_a_missing_or_malformed_status_is_sent_as_server_error() {
    for wire in [
        "5.error,18.Aborted. See logs.;",
        "5.error;",
        "5.error,18.Aborted. See logs.,4.+769;",
        "5.error,18.Aborted. See logs.,6.0x0301;",
        "5.error,18.Aborted. See logs.,4. 769;",
        "5.error,18.Aborted. See logs.,6.000769;",
    ] {
        ends_with_error(wire, "512", Cause::GatewayError);
    }
}

#[test]
fn log_007_guacds_error_text_never_reaches_the_client() {
    ends_with_error(
        "5.error,34.connection 3f2a to 10.0.0.9 failed,3.519;",
        "519",
        Cause::GatewayError,
    );
}

/// guacd 1.5.5 and 1.6.0 send a host-key mismatch as `guac_client_abort`'s fixed text with
/// status 0x0200 (`src/common-ssh/ssh.c` and `src/libguac/client.c`, cited in `gateway.rs`).
/// The words "Host key did not match any provided known host keys." only reach guacd's log,
/// so the wire form is a plain gateway error; the hub checks the key itself (R-034).
#[test]
fn sec_007_guacds_host_key_abort_is_a_gateway_error_on_the_wire() {
    ends_with_error(
        "5.error,18.Aborted. See logs.,3.512;",
        "512",
        Cause::GatewayError,
    );
    ends_with_error(
        "5.error,53.Host key did not match any provided known host keys. ,3.512;",
        "512",
        Cause::GatewayError,
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
