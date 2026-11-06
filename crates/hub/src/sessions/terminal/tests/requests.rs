//! Handing a PTY or shell request to the connection (K4 Section 8.5): a hand-off still
//! waiting at the request wait ends the session `GATEWAY_ERROR`, and the end signal ends it
//! first. Time is paused, so the waits are exact.

use std::time::Duration;

use chrono::TimeDelta;

use super::super::shell::handed;
use crate::sessions::{
    Actor, Live, Presented, Tunnel,
    end::Cause,
    tests::tickets::{CLIENT, client, issue, issued_at, session},
};

/// A redeemed ticket's tunnel on a live state of its own.
fn tunnel() -> (Live, Tunnel) {
    let live = Live::default();
    let text = issue(&live, 1, 4);
    let presented = Presented::terminal(&text, client(CLIENT));
    let tunnel = live.redeem(presented, issued_at()).unwrap().tunnel;
    (live, tunnel)
}

#[tokio::test(start_paused = true)]
async fn k4_8_5_a_request_the_connection_never_takes_is_gateway_error_at_the_wait() {
    let (_live, mut tunnel) = tunnel();
    let started = tokio::time::Instant::now();
    let due = started + Duration::from_secs(15);
    let end = handed(&mut tunnel, std::future::pending(), due)
        .await
        .unwrap_err();
    assert_eq!(end.cause, Cause::GatewayError);
    assert_eq!(
        end.message,
        "The device did not answer the hub's request for a terminal or shell in time, so the \
         hub closed the SSH session."
    );
    assert_eq!(started.elapsed(), Duration::from_secs(15));
}

#[tokio::test(start_paused = true)]
async fn d_10_end_session_ends_a_request_still_being_handed_over() {
    let (live, mut tunnel) = tunnel();
    let by = Actor {
        operator: crate::sessions::tests::fixture::operator(),
        client: client("10.10.0.80"),
        at: issued_at() + TimeDelta::seconds(3),
    };
    live.end_by(session(1), &by);
    let due = tokio::time::Instant::now() + Duration::from_secs(15);
    let end = handed(&mut tunnel, std::future::pending(), due)
        .await
        .unwrap_err();
    assert_eq!((end.cause, end.by), (Cause::OperatorClosed, Some(by)));
}

#[tokio::test(start_paused = true)]
async fn k4_8_5_a_request_taken_in_time_goes_on() {
    let (_live, mut tunnel) = tunnel();
    let due = tokio::time::Instant::now() + Duration::from_secs(15);
    assert!(handed(&mut tunnel, async { Ok(()) }, due).await.is_ok());
}
