//! The pure decisions: each egress failure's reason and code, whether it is permanent, and when
//! a failed notification is tried again (CR-14 item 5, D-16).

use chrono::{DateTime, TimeDelta, Utc};

use crate::fleet::{
    notifications::{
        DeliveryReason,
        dispatch::{
            failure::Failure,
            retry::{Next, WAITS, after_failure, out_of_tries},
        },
        egress::SendError,
    },
    targets::TargetError,
};

fn mapped(error: SendError) -> (DeliveryReason, Option<i32>) {
    let failure = Failure::from(error);
    (failure.reason, failure.code)
}

#[test]
fn cr_14_5_a_refused_target_is_target_not_permitted() {
    assert_eq!(
        mapped(SendError::Target(TargetError::Refused)),
        (DeliveryReason::TargetNotPermitted, None)
    );
}

#[test]
fn cr_14_5_an_unresolved_host_is_target_not_permitted_too() {
    assert_eq!(
        mapped(SendError::Target(TargetError::Unresolved)),
        (DeliveryReason::TargetNotPermitted, None)
    );
}

#[test]
fn cr_14_5_a_url_that_is_not_https_is_bad_address() {
    assert_eq!(mapped(SendError::Url), (DeliveryReason::BadAddress, None));
}

#[test]
fn cr_14_5_a_setup_failure_is_setup_failed() {
    assert_eq!(
        mapped(SendError::Setup),
        (DeliveryReason::SetupFailed, None)
    );
}

#[test]
fn cr_14_5_a_failed_connection_is_connection_failed() {
    assert_eq!(
        mapped(SendError::Connect),
        (DeliveryReason::ConnectionFailed, None)
    );
}

#[test]
fn cr_14_5_a_failed_handshake_is_tls_failed() {
    assert_eq!(mapped(SendError::Tls), (DeliveryReason::TlsFailed, None));
}

#[test]
fn cr_14_5_the_time_limit_is_timed_out() {
    assert_eq!(
        mapped(SendError::TimedOut),
        (DeliveryReason::TimedOut, None)
    );
}

#[test]
fn cr_14_5_a_status_keeps_its_code() {
    assert_eq!(
        mapped(SendError::HttpStatus(503)),
        (DeliveryReason::HttpStatus, Some(503))
    );
}

#[test]
fn cr_14_5_a_missing_starttls_is_no_starttls() {
    assert_eq!(
        mapped(SendError::NoStartTls),
        (DeliveryReason::NoStarttls, None)
    );
}

#[test]
fn cr_14_5_a_refusing_reply_keeps_its_code() {
    assert_eq!(
        mapped(SendError::SmtpReply(550)),
        (DeliveryReason::SmtpRefused, Some(550))
    );
}

#[test]
fn cr_14_5_a_broken_exchange_is_not_understood() {
    assert_eq!(
        mapped(SendError::Exchange),
        (DeliveryReason::NotUnderstood, None)
    );
}

#[test]
fn d_16_what_the_operator_must_change_is_permanent() {
    for reason in [
        DeliveryReason::TargetNotPermitted,
        DeliveryReason::BadAddress,
        DeliveryReason::SecretUnreadable,
        DeliveryReason::SetupFailed,
    ] {
        assert!(Failure::of(reason).is_permanent(), "{reason:?}");
    }
}

#[test]
fn d_16_what_may_pass_is_retried() {
    for reason in [
        DeliveryReason::ConnectionFailed,
        DeliveryReason::TlsFailed,
        DeliveryReason::TimedOut,
        DeliveryReason::NoStarttls,
        DeliveryReason::NotUnderstood,
    ] {
        assert!(!Failure::of(reason).is_permanent(), "{reason:?}");
    }
}

#[test]
fn d_16_a_client_error_is_permanent_but_timeout_and_rate_limit() {
    let permanent = |status| Failure::from(SendError::HttpStatus(status)).is_permanent();
    for status in [400, 401, 403, 404, 410, 413, 422, 499] {
        assert!(permanent(status), "{status}");
    }
    for status in [408, 429, 500, 502, 503, 504, 302, 101] {
        assert!(!permanent(status), "{status}");
    }
}

#[test]
fn d_16_a_permanent_negative_mail_reply_is_permanent_and_a_transient_one_is_not() {
    let permanent = |reply| Failure::from(SendError::SmtpReply(reply)).is_permanent();
    for reply in [500, 535, 550, 554] {
        assert!(permanent(reply), "{reply}");
    }
    for reply in [421, 450, 451, 452] {
        assert!(!permanent(reply), "{reply}");
    }
}

fn at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

#[test]
fn d_16_a_failed_try_waits_1_5_15_and_60_minutes_then_gives_up() {
    let failure = Failure::of(DeliveryReason::TimedOut);
    let waits: Vec<Next> = (1..=5)
        .map(|tried| after_failure(failure, tried, at()))
        .collect();

    assert_eq!(
        waits,
        [
            Next::RetryAt(at() + TimeDelta::minutes(1)),
            Next::RetryAt(at() + TimeDelta::minutes(5)),
            Next::RetryAt(at() + TimeDelta::minutes(15)),
            Next::RetryAt(at() + TimeDelta::minutes(60)),
            Next::GiveUp,
        ]
    );
    assert_eq!(after_failure(failure, u32::MAX, at()), Next::GiveUp);
    assert_eq!(WAITS.len(), 4);
}

#[test]
fn d_16_a_permanent_failure_is_given_up_at_the_first_try() {
    let failure = Failure::from(SendError::HttpStatus(404));
    assert_eq!(after_failure(failure, 1, at()), Next::GiveUp);
}

#[test]
fn d_16_a_sixth_claim_has_no_try_left() {
    assert_eq!(
        (1..=7).map(out_of_tries).collect::<Vec<_>>(),
        [false, false, false, false, false, true, true]
    );
    assert!(out_of_tries(u32::MAX));
}

#[test]
fn d_16_r_103_an_unresolved_host_is_retried() {
    let failure = Failure::from(SendError::Target(TargetError::Unresolved));

    assert_eq!(failure.reason, DeliveryReason::TargetNotPermitted);
    assert!(!failure.is_permanent());
    assert_eq!(
        after_failure(failure, 1, at()),
        Next::RetryAt(at() + TimeDelta::minutes(1))
    );
}

#[test]
fn d_16_r_103_a_host_outside_the_allow_list_is_given_up_at_once() {
    let failure = Failure::from(SendError::Target(TargetError::Refused));

    assert_eq!(failure.reason, DeliveryReason::TargetNotPermitted);
    assert!(failure.is_permanent());
    assert_eq!(after_failure(failure, 1, at()), Next::GiveUp);
}
