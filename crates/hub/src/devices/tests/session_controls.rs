//! The session limit and the recording flag: the rules and the store (RA-036, RA-040).

use std::num::NonZeroU8;

use super::*;
use super::{new_device, record};
use crate::devices::{rules::validate_change, store};

const NOW: &str = "2026-11-02T10:00:00Z";

fn limit(value: Option<i32>) -> NewDeviceInput {
    NewDeviceInput {
        remote: RemoteForm { session_limit: value, ..RemoteForm::default() },
        ..new_device("win-01", "10.10.0.9")
    }
}

fn change(value: Option<i32>) -> DeviceChangeInput {
    DeviceChangeInput {
        remote: RemoteForm { session_limit: value, ..RemoteForm::default() },
        ..DeviceChangeInput::default()
    }
}

fn nz(value: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(value)
}

#[test]
fn ra_036_a_new_device_has_no_limit_unless_one_is_given() {
    let none = validate_new_device(&limit(None)).unwrap();
    assert_eq!((none.remote.session_limit, none.remote.record_sessions), (None, false));
    let set = validate_new_device(&limit(Some(4))).unwrap();
    assert_eq!(set.remote.session_limit, nz(4));
}

#[test]
fn ra_036_a_new_device_refuses_a_limit_outside_1_to_100_on_session_limit() {
    for bad in [0, 101, -1, 256, i32::MAX, i32::MIN] {
        let errors = validate_new_device(&limit(Some(bad))).unwrap_err();
        assert_eq!(fields(&errors), ["sessionLimit"], "{bad}");
        assert_eq!(
            errors[0].message,
            "Enter a number from 1 to 100, or leave it empty for no limit."
        );
    }
    for good in [1, 100] {
        assert!(validate_new_device(&limit(Some(good))).is_ok(), "{good}");
    }
}

#[test]
fn ra_036_an_update_keeps_on_null_clears_on_zero_and_sets_1_to_100() {
    assert_eq!(validate_change(&change(None)).unwrap().remote.session_limit, None);
    assert_eq!(validate_change(&change(Some(0))).unwrap().remote.session_limit, Some(None));
    assert_eq!(validate_change(&change(Some(100))).unwrap().remote.session_limit, Some(nz(100)));
    for bad in [101, -1, i32::MIN] {
        let errors = validate_change(&change(Some(bad))).unwrap_err();
        assert_eq!(fields(&errors), ["sessionLimit"], "{bad}");
    }
}

#[test]
fn ra_040_the_flag_defaults_to_false_and_an_update_without_it_keeps_it() {
    assert!(!validate_new_device(&limit(None)).unwrap().remote.record_sessions);
    let form = RemoteForm { record_sessions: Some(true), ..RemoteForm::default() };
    let created = validate_new_device(&NewDeviceInput { remote: form, ..limit(None) }).unwrap();
    assert!(created.remote.record_sessions);
    assert_eq!(validate_change(&change(None)).unwrap().remote.record_sessions, None);
}

store_test! {
    async fn ra_036_the_limit_and_flag_are_stored_and_read_back(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let mut device = record("limited");
        device.remote.session_limit = nz(3);
        device.remote.record_sessions = true;
        let id = device_id("0190a000-0000-7000-8002-0000000000f2");
        let created = store::create_device(&mut conn, id, instant(NOW), &device).await.unwrap();
        assert_eq!((created.session_limit, created.record_sessions), (nz(3), true));
        assert_eq!(store::get(&pool, id).await.unwrap(), created);
    }

    async fn ra_036_an_update_sets_keeps_and_clears_the_limit_in_the_row(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let id = device_id(PVE_01);
        let set = DeviceChange {
            remote: RemoteChange { session_limit: Some(nz(9)), ..RemoteChange::default() },
            ..DeviceChange::default()
        };
        let after = store::update_device(&mut conn, id, instant(NOW), &set).await.unwrap();
        assert_eq!(after.session_limit, nz(9));
        let keep = DeviceChange { maintenance: Some(true), ..DeviceChange::default() };
        let after = store::update_device(&mut conn, id, instant(NOW), &keep).await.unwrap();
        assert_eq!(after.session_limit, nz(9));
        let clear = DeviceChange {
            remote: RemoteChange { session_limit: Some(None), ..RemoteChange::default() },
            ..DeviceChange::default()
        };
        let after = store::update_device(&mut conn, id, instant(NOW), &clear).await.unwrap();
        assert_eq!(after.session_limit, None);
    }

    async fn ra_040_an_update_sets_and_keeps_the_flag_in_the_row(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let id = device_id(PVE_01);
        let set = DeviceChange {
            remote: RemoteChange { record_sessions: Some(true), ..RemoteChange::default() },
            ..DeviceChange::default()
        };
        assert!(store::update_device(&mut conn, id, instant(NOW), &set).await.unwrap().record_sessions);
        let keep = DeviceChange { maintenance: Some(true), ..DeviceChange::default() };
        assert!(store::update_device(&mut conn, id, instant(NOW), &keep).await.unwrap().record_sessions);
    }

    async fn ra_036_the_database_refuses_a_limit_of_0_or_101(pool: PgPool) {
        for bad in [0, 101] {
            let result = sqlx::query("UPDATE devices.devices SET session_limit = $1")
                .bind(bad)
                .execute(&pool)
                .await;
            assert!(result.is_err(), "{bad}");
        }
    }
}
