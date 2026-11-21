//! The limits of a name (Section 12.7).

use crate::limits::{Limiter, Limits, Refusal};

const LIMITS: Limits = Limits {
    connections: 2,
    new_per_second: 3,
    bytes_per_period: 1000,
    period_ms: 60_000,
};

#[test]
fn connections_at_one_time_are_limited_and_a_release_frees_one() {
    let mut limiter = Limiter::new(LIMITS);
    limiter.admit("a.example.com", 0).unwrap();
    limiter.admit("a.example.com", 0).unwrap();
    assert_eq!(limiter.admit("a.example.com", 0), Err(Refusal::Connections));
    assert_eq!(
        limiter.admit("b.example.com", 0),
        Ok(()),
        "each name has its own limits"
    );
    limiter.release("a.example.com");
    assert_eq!(limiter.admit("a.example.com", 0), Ok(()));
}

#[test]
fn new_connections_each_second_are_limited_and_refill_with_time() {
    let mut limiter = Limiter::new(Limits {
        connections: 100,
        ..LIMITS
    });
    for _ in 0..3 {
        limiter.admit("a.example.com", 0).unwrap();
    }
    assert_eq!(limiter.admit("a.example.com", 100), Err(Refusal::Rate));
    assert_eq!(limiter.admit("a.example.com", 400), Ok(()));
}

#[test]
fn the_bytes_of_a_period_are_limited_until_the_next_period() {
    let mut limiter = Limiter::new(Limits {
        connections: 100,
        new_per_second: 100,
        ..LIMITS
    });
    limiter.admit("a.example.com", 0).unwrap();
    assert!(limiter.add_bytes("a.example.com", 600, 10));
    assert!(!limiter.add_bytes("a.example.com", 600, 20));
    assert_eq!(limiter.admit("a.example.com", 30), Err(Refusal::Bytes));
    assert_eq!(limiter.admit("a.example.com", 60_030), Ok(()));
}
