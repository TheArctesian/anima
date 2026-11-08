//! Notifications (NOT-001 to NOT-007): the channels an operator sets up, the decision of what
//! to send when a check changes state, and the deliveries that send it.
//!
//! Every notification connection leaves the hub through one path, `egress`: the target rule
//! and the deny list judge each resolved address, the connection goes only to a checked
//! address, and TLS is always verified (P3-003, D-17).

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the dispatcher (WP-45 T6) is the first caller")
)]
mod egress;
