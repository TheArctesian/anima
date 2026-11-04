//! The end table (K4 Section 5 as amended by R-032, D-9) and the relay timings.

use std::time::Duration;

use guac_protocol::encode;

use super::super::ending::{CLIENT_IDLE, EndCause, Ending, GATEWAY_SILENCE};
use super::super::relay::tunnel_opened;
use crate::events::Severity;
use crate::sessions::EndReason;

fn row(close_code: u16, reason: EndReason, severity: Severity) -> Ending {
    Ending {
        close_code,
        reason,
        severity,
    }
}

#[test]
fn k4_operator_and_client_close_end_1000_operator_info() {
    let expected = row(1000, EndReason::Operator, Severity::Info);
    assert_eq!(EndCause::OperatorClosed.ending(), expected);
}

#[test]
fn k4_remote_close_ends_1000_remote_closed_info() {
    let expected = row(1000, EndReason::RemoteClosed, Severity::Info);
    assert_eq!(EndCause::RemoteClosed.ending(), expected);
}

#[test]
fn k4_gateway_error_ends_1011_warning() {
    let expected = row(1011, EndReason::GatewayError, Severity::Warning);
    assert_eq!(EndCause::GatewayError.ending(), expected);
}

#[test]
fn k4_auth_failure_ends_1011_warning() {
    let expected = row(1011, EndReason::AuthFailed, Severity::Warning);
    assert_eq!(EndCause::AuthFailed.ending(), expected);
}

#[test]
fn sec_007_host_key_mismatch_ends_4001_error() {
    let expected = row(4001, EndReason::HostKeyMismatch, Severity::Error);
    assert_eq!(EndCause::HostKeyMismatch.ending(), expected);
}

#[test]
fn k4_idle_timeout_ends_4002_timeout_info() {
    let expected = row(4002, EndReason::Timeout, Severity::Info);
    assert_eq!(EndCause::Timeout.ending(), expected);
}

#[test]
fn k4_hub_shutdown_ends_1001_hub_shutdown_info() {
    let expected = row(1001, EndReason::HubShutdown, Severity::Info);
    assert_eq!(EndCause::HubShutdown.ending(), expected);
}

#[test]
fn gw_011_an_invalid_instruction_ends_1002_operator_warning() {
    let expected = row(1002, EndReason::Operator, Severity::Warning);
    assert_eq!(EndCause::InvalidInstruction.ending(), expected);
}

#[test]
fn k4_relay_timings_are_5_and_15_seconds() {
    assert_eq!(GATEWAY_SILENCE, Duration::from_secs(5));
    assert_eq!(CLIENT_IDLE, Duration::from_secs(15));
}

#[test]
fn gw_014_the_open_instruction_carries_only_the_tunnel_id() {
    let id = "8e8c9a5e-6f0b-4b0e-9d58-3c4bfb0b7e21";
    assert_eq!(encode(&tunnel_opened(id)), format!("0.,36.{id};"));
}
