//! The RDP side of the gateway.

mod active;
/// The RDP device of the tests, shared with the hub's tunnel tests.
pub(crate) use rdp_test_device as device;
mod keys;
mod limits;
mod opening;
mod pointer;
mod sharing;
