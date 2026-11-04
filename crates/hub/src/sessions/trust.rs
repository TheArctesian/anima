//! Trusting a device's SSH host key (SEC-006, DEV-013): the connection test, the keys it saw,
//! and the operator accepting one of them (D-5, D-6).

mod accept;
mod connection_test;
mod observations;

pub(super) use accept::{Acceptance, accept_host_key};
pub(super) use connection_test::{
    ConnectionTest, TestRequest, TestSubject, Unsaved, test_connection,
};
pub(super) use observations::{Endpoint, Observation, Observations};
#[cfg(test)]
pub(super) use observations::{LIFETIME, MAX_OBSERVATIONS};
