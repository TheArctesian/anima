//! The host keys connection tests saw (D-5): kept 15 minutes per host and port from the
//! injected time, bounded in number, and one spelling per endpoint.

use chrono::{DateTime, TimeDelta, Utc};

use super::{ECDSA, ED25519};
use crate::sessions::{
    live::Live,
    public_key::PublicKey,
    trust::{Endpoint, LIFETIME, MAX_OBSERVATIONS, Observations},
};

fn at() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn key(text: &str) -> PublicKey {
    PublicKey::from_base64(text).unwrap()
}

fn pve() -> Endpoint {
    Endpoint::new("pve-01.lan", 22)
}

#[test]
fn dev_013_an_observation_lasts_fifteen_minutes_from_when_the_key_was_last_seen() {
    let live = Live::default();
    live.observe(pve(), key(ED25519), at());
    let last_moment = at() + LIFETIME - TimeDelta::nanoseconds(1);
    assert_eq!(
        live.observed(&pve(), last_moment).unwrap().key,
        key(ED25519)
    );
    assert_eq!(
        live.observed(&pve(), at() + LIFETIME),
        None,
        "boundary is gone"
    );

    // Seeing the same key again keeps when it was first seen and extends its life.
    let again = at() + TimeDelta::minutes(10);
    let seen = live.observe(pve(), key(ED25519), again);
    assert_eq!((seen.first_seen_at, seen.last_seen_at), (at(), again));
    assert!(live.observed(&pve(), at() + LIFETIME).is_some());
    assert_eq!(live.observed(&pve(), again + LIFETIME), None);
}

#[test]
fn sec_006_another_key_at_the_same_endpoint_replaces_the_first_and_its_first_seen_time() {
    let live = Live::default();
    live.observe(pve(), key(ED25519), at());
    let later = at() + TimeDelta::minutes(1);
    let seen = live.observe(pve(), key(ECDSA), later);
    assert_eq!((seen.key.clone(), seen.first_seen_at), (key(ECDSA), later));
    assert_eq!(live.observed(&pve(), later), Some(seen));
}

#[test]
fn dev_013_an_endpoint_is_one_spelling_of_host_and_port() {
    assert_eq!(Endpoint::new(" PVE-01.lan. ", 22), pve());
    assert_eq!(
        Endpoint::new("2001:DB8:0::1", 22),
        Endpoint::new("2001:db8::1", 22)
    );
    assert_ne!(Endpoint::new("pve-01.lan", 2222), pve());
    assert_ne!(Endpoint::new("pve-02.lan", 22), pve());
    let live = Live::default();
    live.observe(Endpoint::new("PVE-01.LAN", 22), key(ED25519), at());
    assert!(live.observed(&pve(), at()).is_some());
    assert_eq!(
        live.observed(&Endpoint::new("pve-01.lan", 2222), at()),
        None
    );
}

#[test]
fn dev_013_observations_are_bounded_and_the_oldest_goes_first() {
    let mut observations = Observations::default();
    for n in 0..=MAX_OBSERVATIONS {
        let minute = TimeDelta::seconds(i64::try_from(n).unwrap());
        observations.record(
            Endpoint::new(&format!("host-{n}.lan"), 22),
            key(ED25519),
            at() + minute,
        );
    }
    assert_eq!(observations.len(), MAX_OBSERVATIONS);
    let now = at() + TimeDelta::minutes(5);
    assert_eq!(
        observations.find(&Endpoint::new("host-0.lan", 22), now),
        None
    );
    assert!(
        observations
            .find(&Endpoint::new("host-1.lan", 22), now)
            .is_some()
    );
    let newest = format!("host-{MAX_OBSERVATIONS}.lan");
    assert!(
        observations
            .find(&Endpoint::new(&newest, 22), now)
            .is_some()
    );
}

#[test]
fn dev_013_expired_observations_make_room_before_a_live_one_is_dropped() {
    let mut observations = Observations::default();
    for n in 0..MAX_OBSERVATIONS {
        let endpoint = Endpoint::new(&format!("host-{n}.lan"), 22);
        // host-0 is old and expired by the time the new one comes; the rest are fresh.
        let seen_at = if n == 0 { at() - LIFETIME } else { at() };
        observations.record(endpoint, key(ED25519), seen_at);
    }
    observations.record(Endpoint::new("new.lan", 22), key(ECDSA), at());
    assert_eq!(observations.len(), MAX_OBSERVATIONS);
    assert!(
        (1..MAX_OBSERVATIONS).all(|n| observations
            .find(&Endpoint::new(&format!("host-{n}.lan"), 22), at())
            .is_some()),
        "no fresh observation was dropped"
    );
}
