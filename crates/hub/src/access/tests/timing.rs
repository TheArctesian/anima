//! An unknown username costs one password check, like a known one (D-8).

use std::{
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};

use sqlx::PgPool;

use super::now;
use crate::{
    access::{
        ClientAddr, Error, Password,
        login::{self, Attempt},
        password::{self, DUMMY_HASH},
    },
    events::Bus,
};

fn attempt(username: &str) -> Attempt {
    Attempt {
        username: username.to_owned(),
        password: Password::new("correct horse battery staple".to_owned()),
        totp_code: None,
        client: ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 9))),
        at: now(),
    }
}

/// The shortest of three runs, to damp scheduler noise.
fn shortest(runs: [Duration; 3]) -> Duration {
    runs.into_iter().min().expect("three runs")
}

#[sqlx::test(migrations = "./migrations")]
async fn d_8_an_unknown_username_still_runs_one_password_check(pool: PgPool) {
    let typed = Password::new("correct horse battery staple".to_owned());
    let mut checks = [Duration::ZERO; 3];
    for run in &mut checks {
        let started = Instant::now();
        assert!(!password::matches(&typed, DUMMY_HASH).unwrap());
        *run = started.elapsed();
    }

    let bus = Bus::new(4);
    let mut logins = [Duration::ZERO; 3];
    for (run, name) in logins.iter_mut().zip(["nobody", "noone", "nemo"]) {
        let started = Instant::now();
        let result = login::login(&pool, &bus, attempt(name)).await;
        *run = started.elapsed();
        assert!(matches!(result, Err(Error::WrongCredentials)));
    }

    let (check, login) = (shortest(checks), shortest(logins));
    assert!(
        login >= check / 2,
        "an unknown username took {login:?}, one check takes {check:?}"
    );
}
