//! Tests of the K8 bodies, the secrets and the batch check.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{Accepted, Batch, RefusalCode};

mod bodies;
mod drives;
mod range;
mod secrets;
mod series;
mod shape;

/// The K8 Section 4 example batch.
const BATCH: &str = include_str!("tests/k8/batch.json");

/// The device of the K8 examples.
fn device() -> Uuid {
    "0192f5c4-6f1e-7a3b-9c2d-5e8f1a2b3c4d".parse().unwrap()
}

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

/// The hub's clock for the example batch: 10 seconds after its newest point.
fn now() -> DateTime<Utc> {
    at("2026-11-08T10:00:30Z")
}

fn example() -> Batch {
    serde_json::from_str(BATCH).unwrap()
}

/// The example batch changed by `change`, checked for the example device at [`now`].
fn checked(change: impl FnOnce(&mut Batch)) -> Result<Accepted, RefusalCode> {
    let mut batch = example();
    change(&mut batch);
    batch.check(device(), now())
}

/// Asserts the example changed by `change` is refused as malformed, while the example is not.
fn assert_malformed(case: &str, change: impl FnOnce(&mut Batch)) {
    assert!(checked(|_| {}).is_ok(), "the example is accepted");
    assert_eq!(
        checked(change).err(),
        Some(RefusalCode::Malformed),
        "{case}"
    );
}

/// Asserts the example changed by `change` is still accepted.
fn assert_accepted(case: &str, change: impl FnOnce(&mut Batch)) {
    let result = checked(change);
    assert!(result.is_ok(), "{case}: {:?}", result.err());
}
