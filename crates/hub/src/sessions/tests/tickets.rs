//! Ticket rules and the in-memory table (K4 §1, API-009, LOG-007), and the end signal of an
//! open tunnel (D-10). Deterministic: every time comes from a `FixedClock`.

use std::{net::IpAddr, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use proptest::prelude::*;

use crate::{
    clock::{Clock, FixedClock},
    events::{ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
    sessions::{
        EndReason, Protocol,
        live::{Live, MAX_PENDING, Refused, Unavailable},
        tickets::{LIFETIME, PendingTicket, TOKEN_BYTES, Ticket, TicketToken, hash_of},
    },
};

const CLIENT: &str = "10.10.0.77";

fn issued_at() -> DateTime<Utc> {
    FixedClock("2026-11-04T09:00:00Z".parse().unwrap()).now()
}

fn client(text: &str) -> ClientAddr {
    ClientAddr(text.parse::<IpAddr>().unwrap())
}

fn session(n: u128) -> SessionId {
    SessionId::from(uuid::Uuid::from_u128(
        0x0190_a000_0000_7000_8010_0000_0000_0000 + n,
    ))
}

fn pending(n: u128) -> PendingTicket {
    PendingTicket {
        operator: Operator {
            id: "0190a000-0000-7000-8000-000000000001"
                .parse::<OperatorId>()
                .unwrap(),
            name: "operator".to_owned(),
        },
        session: session(n),
        client: client(CLIENT),
        device: "0190a000-0000-7000-8002-000000000001"
            .parse::<DeviceId>()
            .unwrap(),
        protocol: Protocol::Ssh,
        display: guac_protocol::Display {
            width: 1280,
            height: 800,
            dpi: 96,
        },
        timezone: Some("Europe/Berlin".to_owned()),
        expires_at: issued_at() + LIFETIME,
    }
}

/// A token made from `seed`, so each test's tokens are fixed and distinct.
fn token(seed: u8) -> TicketToken {
    TicketToken::from_random([seed; TOKEN_BYTES])
}

/// Issues ticket `n` with the token from `seed` and returns the token's text.
fn issue(live: &Live, n: u128, seed: u8) -> String {
    let token = token(seed);
    live.issue(token.hash(), pending(n)).unwrap();
    token.expose().to_owned()
}

#[test]
fn api_009_a_token_is_43_characters_of_unpadded_base64url() {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).unwrap();
    let token = TicketToken::from_random(bytes);
    assert_eq!(token.expose().len(), 43);
    assert!(
        token
            .expose()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    );
    assert!(hash_of(token.expose()) == Some(token.hash()));
}

#[test]
fn api_009_text_that_is_not_one_canonical_token_has_no_hash() {
    let text = token(7).expose().to_owned();
    assert!(hash_of(&text[..42]).is_none());
    assert!(hash_of(&format!("{text}=")).is_none());
    assert!(hash_of(&format!("+{}", &text[1..])).is_none());
    // The last character carries 2 unused bits; a spelling that sets them is refused.
    let last = text.chars().last().unwrap();
    let mut other = text[..42].to_owned();
    other.push(if last == 'B' { 'C' } else { 'B' });
    assert!(hash_of(&other).is_none());
}

proptest! {
    /// K4 §1: refused when `now >= expires_at` (the boundary refuses) or from another
    /// address; refused or not, the ticket is gone after one presentation.
    #[test]
    fn api_009_redeem_admits_only_before_expiry_from_the_issuing_address(
        offset_ms in -40_000_i64..40_000,
        other_client in any::<bool>(),
        last_octet in 1_u8..=254,
    ) {
        let live = Live::default();
        let text = issue(&live, 1, 9);
        let expires_at = pending(1).expires_at;
        let now = expires_at + TimeDelta::milliseconds(offset_ms);
        let from = if other_client && last_octet != 77 {
            client(&format!("10.10.0.{last_octet}"))
        } else {
            client(CLIENT)
        };
        let admitted = now < expires_at && from == client(CLIENT);

        let first = live.redeem(&text, from, now);
        prop_assert_eq!(first.is_ok(), admitted);
        if let Ok(redeemed) = &first {
            prop_assert_eq!(&redeemed.ticket, &pending(1));
        }
        // Single use: valid or not, a second presentation is refused, even in time from the
        // right address. A refused ticket waits, without its token, for the sweep to end its
        // row once expired; an admitted one left the table for the tunnel registry.
        prop_assert_eq!(live.redeem(&text, client(CLIENT), issued_at()).err(), Some(Refused));
        let swept = live.take_expired(expires_at);
        prop_assert_eq!(swept.len(), usize::from(!admitted));
        prop_assert!(live.take_pending(session(1)).is_none());
    }

    /// An unknown or malformed token is refused and leaves the issued ticket in place.
    #[test]
    fn api_009_an_unknown_token_is_refused_and_takes_nothing(text in "[A-Za-z0-9_-]{0,50}") {
        let live = Live::default();
        let issued = issue(&live, 1, 9);
        prop_assume!(text != issued);
        prop_assert_eq!(live.redeem(&text, client(CLIENT), issued_at()).err(), Some(Refused));
        prop_assert!(live.redeem(&issued, client(CLIENT), issued_at()).is_ok());
    }
}

#[test]
fn api_009_the_expiry_instant_itself_is_refused() {
    let live = Live::default();
    let text = issue(&live, 1, 3);
    let at_expiry = FixedClock(pending(1).expires_at).now();
    assert_eq!(
        live.redeem(&text, client(CLIENT), at_expiry).err(),
        Some(Refused)
    );
}

#[test]
fn log_007_no_ticket_type_or_refusal_shows_the_token() {
    let token = token(5);
    let text = token.expose().to_owned();
    let hash = token.hash();
    let ticket = Ticket::new(session(1), token, pending(1).expires_at);
    let shown = format!(
        "{ticket:?} {:?} {Refused} {Refused:?} {Unavailable}",
        pending(1)
    );
    assert!(!shown.contains(&text), "{shown}");
    assert!(shown.contains("[redacted]"));
    assert_eq!(ticket.token(), text);
    assert!(hash_of(&text) == Some(hash));
}

#[test]
fn api_009_take_expired_returns_only_tickets_expired_at_now() {
    let live = Live::default();
    issue(&live, 1, 1);
    let late = live.issue(
        token(2).hash(),
        PendingTicket {
            expires_at: pending(2).expires_at + TimeDelta::seconds(10),
            ..pending(2)
        },
    );
    late.unwrap();
    let expired = live.take_expired(pending(1).expires_at);
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].session, session(1));
    live.restore(expired);
    assert_eq!(
        live.take_expired(pending(1).expires_at + TimeDelta::seconds(10))
            .len(),
        2
    );
}

#[test]
fn api_009_the_pending_table_is_bounded() {
    let live = Live::default();
    for n in 0..MAX_PENDING {
        let mut bytes = [0; TOKEN_BYTES];
        bytes[..8].copy_from_slice(&(n as u64).to_le_bytes());
        let token = TicketToken::from_random(bytes);
        live.issue(token.hash(), pending(n as u128)).unwrap();
    }
    assert_eq!(
        live.issue(token(255).hash(), pending(999)),
        Err(Unavailable)
    );
    assert!(live.take_pending(session(0)).is_some());
    assert!(live.issue(token(255).hash(), pending(999)).is_ok());
}

#[tokio::test]
async fn ra_008_end_live_signals_operator_and_finishes_when_the_tunnel_drops() {
    let live = Live::default();
    let text = issue(&live, 1, 4);
    let mut tunnel = live
        .redeem(&text, client(CLIENT), issued_at())
        .unwrap()
        .tunnel;

    let closing = live.end_live(session(1)).expect("the tunnel is registered");
    assert_eq!(tunnel.ended().await, EndReason::Operator);
    let finished = tokio::spawn(closing.finished());
    tokio::task::yield_now().await;
    assert!(!finished.is_finished(), "waits for the tunnel to drop");
    drop(tunnel);
    tokio::time::timeout(Duration::from_secs(5), finished)
        .await
        .unwrap()
        .unwrap();
    assert!(live.end_live(session(1)).is_none(), "dropping deregisters");
}

#[tokio::test]
async fn ra_008_shut_down_signals_hub_shutdown_to_every_tunnel_and_refuses_new_ones() {
    let live = Live::default();
    let first = issue(&live, 1, 1);
    let second = issue(&live, 2, 2);
    let pending_third = issue(&live, 3, 3);
    let mut one = live
        .redeem(&first, client(CLIENT), issued_at())
        .unwrap()
        .tunnel;
    let mut two = live
        .redeem(&second, client(CLIENT), issued_at())
        .unwrap()
        .tunnel;

    let closing = live.shut_down();
    assert_eq!(one.ended().await, EndReason::HubShutdown);
    assert_eq!(two.ended().await, EndReason::HubShutdown);
    // The first cause wins: a later endSession does not change it.
    drop(live.end_live(session(1)));
    assert_eq!(one.ended().await, EndReason::HubShutdown);
    assert_eq!(
        live.redeem(&pending_third, client(CLIENT), issued_at())
            .err(),
        Some(Refused)
    );
    assert_eq!(live.issue(token(9).hash(), pending(9)), Err(Unavailable));
    drop((one, two));
    tokio::time::timeout(Duration::from_secs(5), closing.finished())
        .await
        .unwrap();
}
