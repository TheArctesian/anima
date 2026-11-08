//! CR-15, K8 Sections 3 and 6: a refusal with no valid credential is written at most once a
//! minute per client address, in bounded memory.

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, Utc};

use crate::metrics::ingest::throttle::{ADDRESSES_PER_SPAN, Throttle};

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000 + seconds, 0).unwrap()
}

fn address(last: u32) -> IpAddr {
    IpAddr::V4(Ipv4Addr::from(0x0a00_0000 + last))
}

#[test]
fn cr_15_an_address_is_written_once_a_minute() {
    let mut throttle = Throttle::default();
    assert!(throttle.admit(address(1), at(0)));
    assert!(!throttle.admit(address(1), at(1)));
    assert!(!throttle.admit(address(1), at(59)));
    assert!(throttle.admit(address(1), at(60)));
    assert!(!throttle.admit(address(1), at(61)));
}

#[test]
fn cr_15_each_address_has_its_own_minute() {
    let mut throttle = Throttle::default();
    assert!(throttle.admit(address(1), at(0)));
    assert!(throttle.admit(address(2), at(10)));
    assert!(!throttle.admit(address(2), at(30)));
    assert!(throttle.admit(address(1), at(60)));
    assert!(!throttle.admit(address(2), at(69)));
    assert!(throttle.admit(address(2), at(70)));
}

#[test]
fn cr_15_a_clock_that_went_back_starts_the_minute_again() {
    let mut throttle = Throttle::default();
    assert!(throttle.admit(address(1), at(100)));
    assert!(throttle.admit(address(1), at(50)));
}

#[test]
fn cr_15_a_flood_of_addresses_keeps_memory_bounded_and_counts_what_it_left_out() {
    let mut throttle = Throttle::default();
    let flood = u32::try_from(ADDRESSES_PER_SPAN).unwrap() + 10;
    let written = (0..flood)
        .filter(|last| throttle.admit(address(*last), at(0)))
        .count();
    assert_eq!(written, ADDRESSES_PER_SPAN);
    assert_eq!(throttle.len(), ADDRESSES_PER_SPAN);
    assert_eq!(throttle.unrecorded(at(59)), None);
    assert_eq!(throttle.unrecorded(at(60)), Some(10));
    assert_eq!(throttle.unrecorded(at(61)), None);
    assert!(throttle.admit(address(flood + 1), at(60)));
    assert_eq!(throttle.len(), 1);
}
