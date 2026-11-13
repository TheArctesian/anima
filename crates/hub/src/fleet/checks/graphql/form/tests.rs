//! The push token of a check form is drawn only for a valid push input (CQ-021, API-010).

use std::cell::Cell;

use async_graphql::{ID, MaybeUndefined};

use super::{create, update};
use crate::fleet::checks::{
    Definition, Error, PushToken,
    graphql::input::{ConfigShape, CreateRetries, CreateShape, IcmpShape, Opaque, UpdateShape},
};

const DEVICE: &str = "0190a7f4-0000-7000-8000-000000000001";

fn create_input(device: &str, config: ConfigShape) -> CreateShape {
    CreateShape {
        device_id: ID::from(device),
        parent_id: None,
        name: "Push".to_owned(),
        config: Opaque(config),
        interval_seconds: Some(60),
        retries: CreateRetries(Some(3)),
        retry_interval_seconds: Some(20),
        degraded_threshold_ms: Some(1000),
        invert: Some(false),
    }
}

fn update_input(config: Option<ConfigShape>) -> UpdateShape {
    UpdateShape {
        parent_id: MaybeUndefined::Undefined,
        name: None,
        config: config.map(Opaque),
        interval_seconds: None,
        retries: None,
        retry_interval_seconds: None,
        degraded_threshold_ms: None,
        invert: None,
    }
}

/// A token source that counts its draws and fails, as an OS random source can.
fn failing(draws: &Cell<u32>) -> impl FnOnce() -> Result<PushToken, Error> + '_ {
    || {
        draws.set(draws.get() + 1);
        Err(Error::Random(getrandom::Error::UNSUPPORTED))
    }
}

#[test]
fn cq_021_a_field_error_is_reported_before_the_token_is_drawn() {
    let draws = Cell::new(0);
    let input = create_input("not-an-id", ConfigShape::Push(true));
    let refused = create(input, failing(&draws)).err();
    assert!(matches!(refused, Some(Error::Validation(fields)) if fields[0].field == "deviceId"));
    assert_eq!(draws.get(), 0);
}

#[test]
fn cq_021_only_a_push_check_draws_a_token() {
    let draws = Cell::new(0);
    let icmp = ConfigShape::Icmp(IcmpShape {
        timeout_ms: None,
        ip_version: None,
    });
    assert!(create(create_input(DEVICE, icmp), failing(&draws)).is_ok());
    let form = update(update_input(None), failing(&draws)).ok().unwrap();
    assert!(form.is_empty());
    assert_eq!(draws.get(), 0);
}

#[test]
fn cq_023_a_failed_draw_for_a_valid_push_input_is_the_random_error() {
    let draws = Cell::new(0);
    let refused = create(
        create_input(DEVICE, ConfigShape::Push(true)),
        failing(&draws),
    )
    .err();
    assert!(matches!(refused, Some(Error::Random(_))));
    let refused = update(update_input(Some(ConfigShape::Push(true))), failing(&draws)).err();
    assert!(matches!(refused, Some(Error::Random(_))));
    assert_eq!(draws.get(), 2);
}

#[test]
fn chk_002_a_valid_push_input_takes_the_drawn_token() {
    let token = PushToken::from_random(&[4; 32]);
    let form = create(create_input(DEVICE, ConfigShape::Push(true)), || {
        Ok(token.clone())
    })
    .ok()
    .unwrap();
    assert!(matches!(form.new.definition, Definition::Push(drawn) if drawn == token));
    let form = update(update_input(Some(ConfigShape::Push(true))), || {
        Ok(token.clone())
    })
    .ok()
    .unwrap();
    assert!(form.rotated);
}
