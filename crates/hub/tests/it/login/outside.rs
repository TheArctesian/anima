//! Failures from outside sign-in (a `joinShare` miss): they count toward the login throttle of
//! their address, by the same rule `login` reads, and never name an account (D-8, CR-16).

use anima_hub::access::{self, ClientAddr, Password};
use chrono::TimeDelta;
use sqlx::PgPool;

use crate::hub::{CLIENT, Hub, PASSWORD, USERNAME, error, set_operator, start, wrong_credentials};

fn address(text: &str) -> ClientAddr {
    ClientAddr(text.parse().unwrap())
}

/// Whether `login` with the right password is refused at `now`.
async fn login_refused(pool: &PgPool, now: chrono::DateTime<chrono::Utc>) -> bool {
    Hub::new(pool, now).login(USERNAME, PASSWORD).await["data"]["login"].is_null()
}

/// Four wrong passwords and one failure from outside sign-in throttle the address, so the
/// operator's own sign-in from there is refused; another address is not throttled.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_failure_outside_sign_in_counts_toward_the_throttle(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for _ in 0..4 {
        hub.login(USERNAME, "wrong password").await;
    }
    let (here, elsewhere) = (address(CLIENT), address("100.64.0.99"));
    assert!(!access::throttled(&pool, here, start()).await.unwrap());

    access::record_failure(&pool, here, start()).await.unwrap();

    assert!(access::throttled(&pool, here, start()).await.unwrap());
    assert!(!access::throttled(&pool, elsewhere, start()).await.unwrap());
    assert_eq!(
        error(&hub.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );
    let other = Hub::from_client(&pool, start(), "100.64.0.99");
    assert!(other.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}

/// Five failures outside sign-in lock the address for 15 minutes after the fifth, and
/// `throttled` and `login` agree at each edge of the window and the lockout.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_throttled_and_login_agree_at_every_edge(pool: PgPool) {
    set_operator(&pool).await;
    let here = address(CLIENT);
    for minute in [0, 1, 2, 3, 15] {
        let at = start() + TimeDelta::minutes(minute);
        access::record_failure(&pool, here, at).await.unwrap();
    }
    for (offset, locked) in [
        (TimeDelta::minutes(15), true),
        (TimeDelta::minutes(30) - TimeDelta::seconds(1), true),
        (TimeDelta::minutes(30), false),
    ] {
        let now = start() + offset;
        assert_eq!(
            access::throttled(&pool, here, now).await.unwrap(),
            locked,
            "throttled at +{offset}"
        );
        assert_eq!(
            login_refused(&pool, now).await,
            locked,
            "login at +{offset}"
        );
    }
}

/// Five failures spread over just more than 15 minutes lock nothing.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_failures_over_more_than_15_minutes_do_not_throttle(pool: PgPool) {
    set_operator(&pool).await;
    let here = address(CLIENT);
    for second in [0, 60, 120, 180, 15 * 60 + 1] {
        let at = start() + TimeDelta::seconds(second);
        access::record_failure(&pool, here, at).await.unwrap();
    }
    let now = start() + TimeDelta::minutes(16);
    assert!(!access::throttled(&pool, here, now).await.unwrap());
    assert!(!login_refused(&pool, now).await);
}

/// Sign-ins refused during a lockout do not extend it, and successes never count.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_only_checked_failures_count(pool: PgPool) {
    set_operator(&pool).await;
    let here = address(CLIENT);
    for _ in 0..5 {
        assert!(!login_refused(&pool, start()).await);
    }
    assert!(!access::throttled(&pool, here, start()).await.unwrap());

    for _ in 0..5 {
        access::record_failure(&pool, here, start()).await.unwrap();
    }
    for minute in 1..15 {
        let hub = Hub::new(&pool, start() + TimeDelta::minutes(minute));
        hub.login(USERNAME, "wrong password").await;
    }
    let released = start() + TimeDelta::minutes(15);
    assert!(!access::throttled(&pool, here, released).await.unwrap());
}

/// An IPv4 address seen as IPv4-mapped IPv6 is the same address (D-11).
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_mapped_address_counts_as_its_ipv4_address(pool: PgPool) {
    for _ in 0..5 {
        access::record_failure(&pool, address("::ffff:100.64.0.7"), start())
            .await
            .unwrap();
    }
    assert!(
        access::throttled(&pool, address(CLIENT), start())
            .await
            .unwrap()
    );
}

/// The same address read the other way: failures from the IPv4 address throttle it when it is
/// seen as IPv4-mapped IPv6 (D-11).
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_11_an_ipv4_address_counts_toward_its_mapped_address(pool: PgPool) {
    for _ in 0..5 {
        access::record_failure(&pool, address(CLIENT), start())
            .await
            .unwrap();
    }
    let mapped = address("::ffff:100.64.0.7");
    assert!(access::throttled(&pool, mapped, start()).await.unwrap());
}

/// A failure outside sign-in recorded during a lockout does not extend it, even when the
/// caller did not ask `throttled` first.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_failure_outside_sign_in_during_a_lockout_does_not_extend_it(pool: PgPool) {
    let here = address(CLIENT);
    for _ in 0..5 {
        access::record_failure(&pool, here, start()).await.unwrap();
    }
    for minute in 1..15 {
        let at = start() + TimeDelta::minutes(minute);
        access::record_failure(&pool, here, at).await.unwrap();
    }
    let released = start() + TimeDelta::minutes(15);
    assert!(!access::throttled(&pool, here, released).await.unwrap());
}

/// A failure dated after `now` does not count at `now`.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_later_failure_does_not_count_now(pool: PgPool) {
    let here = address(CLIENT);
    let later = start() + TimeDelta::minutes(1);
    for _ in 0..5 {
        access::record_failure(&pool, here, later).await.unwrap();
    }
    assert!(access::throttled(&pool, here, later).await.unwrap());
    assert!(!access::throttled(&pool, here, start()).await.unwrap());
}

/// The username a failure outside sign-in is stored under can never be an operator's, so it
/// never matches or locks a real account.
#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_failure_outside_sign_in_names_no_account(pool: PgPool) {
    access::record_failure(&pool, address(CLIENT), start())
        .await
        .unwrap();
    let marker: String = sqlx::query_scalar("select username from access.login_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();

    let refused = access::set_operator(&pool, &marker, Password::new(PASSWORD.to_owned()), start());
    assert!(
        matches!(refused.await, Err(access::Error::Validation { .. })),
        "{marker} could name an operator"
    );
}
