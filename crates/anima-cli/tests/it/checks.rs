//! `anima check` against the scripted hub: reads in `read`, changes in `write`.

mod read;
mod write;

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::FakeHub;

const TOKEN: &str = "login-token-for-check-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const CHECK: &str = "0190a000-0000-7000-8003-000000000002";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

/// The operations the hub was sent, in order.
fn operations(hub: &FakeHub) -> Vec<String> {
    hub.seen().into_iter().map(|seen| seen.operation).collect()
}

/// The variables of the first request for `operation`.
fn variables(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .map_or(json!(null), |seen| seen.variables)
}
