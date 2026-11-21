//! The proof of a name (Section 12.4, SF-NEW-routetoken): the token names this hub, and one
//! failed read removes nothing.

use super::support::{FakeDns, HUB, name};
use crate::work::routes::proof::{
    LOST_AFTER, Proof, ProofRule, Read, after, new_token, read, record_value,
};

fn token_rule() -> ProofRule {
    ProofRule::Token {
        hub: HUB.to_owned(),
    }
}

#[test]
fn a_token_is_256_random_bits_in_hex_and_two_differ() {
    let first = new_token().unwrap();
    assert_eq!(first.len(), 64);
    assert!(
        first
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_ne!(first, new_token().unwrap());
}

#[tokio::test]
async fn the_record_holds_only_with_this_hub_and_this_token() {
    let dns = FakeDns::default();
    let photos = name("photos.example.com");
    let token = "ab".repeat(32);
    assert_eq!(
        read(&dns, &token_rule(), &photos, &token).await,
        Read::DoesNotHold
    );
    dns.set_txt(
        "_anima.photos.example.com",
        &[record_value("hub.other.ing", &token)],
    );
    assert_eq!(
        read(&dns, &token_rule(), &photos, &token).await,
        Read::DoesNotHold
    );
    dns.set_txt("_anima.photos.example.com", &[record_value(HUB, &token)]);
    assert_eq!(
        read(&dns, &token_rule(), &photos, &token).await,
        Read::Holds
    );
    dns.fail("_anima.photos.example.com");
    assert_eq!(
        read(&dns, &token_rule(), &photos, &token).await,
        Read::Failed
    );
}

#[tokio::test]
async fn by_address_each_address_must_be_the_hub() {
    let mut dns = FakeDns::default();
    let rule = ProofRule::Address("203.0.113.5".parse().unwrap());
    let photos = name("photos.example.com");
    assert_eq!(read(&dns, &rule, &photos, "").await, Read::DoesNotHold);
    dns.addresses.insert(
        "photos.example.com".to_owned(),
        vec![
            "203.0.113.5".parse().unwrap(),
            "198.51.100.7".parse().unwrap(),
        ],
    );
    assert_eq!(read(&dns, &rule, &photos, "").await, Read::DoesNotHold);
    dns.addresses.insert(
        "photos.example.com".to_owned(),
        vec!["203.0.113.5".parse().unwrap()],
    );
    assert_eq!(read(&dns, &rule, &photos, "").await, Read::Holds);
}

#[test]
fn a_proven_name_is_lost_only_after_several_failed_reads_in_a_row() {
    let mut state = (Proof::Proven, 0);
    for _ in 1..LOST_AFTER {
        state = after(state.0, state.1, Read::Failed);
        assert_eq!(state.0, Proof::Proven);
    }
    assert_eq!(after(state.0, state.1, Read::Holds), (Proof::Proven, 0));
    let lost = after(state.0, state.1, Read::DoesNotHold);
    assert_eq!(lost, (Proof::Lost, LOST_AFTER));
}

#[test]
fn a_waiting_name_stays_waiting_until_it_holds() {
    assert_eq!(
        after(Proof::Waiting, 7, Read::DoesNotHold),
        (Proof::Waiting, 8)
    );
    assert_eq!(after(Proof::Lost, 4, Read::Holds), (Proof::Proven, 0));
}
