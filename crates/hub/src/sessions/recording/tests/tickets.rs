//! Playback tickets (K4 §1, §12, G-7, API-009): held by hash only, single use, 30 seconds,
//! bound to the client address, and bounded in number. Pure: the time is a parameter.

use chrono::TimeDelta;
use uuid::Uuid;

use crate::{
    ids::{RecordingId, SessionId},
    sessions::{
        Refused,
        recording::tickets::{MAX_PENDING, PlaybackTicket, Tickets},
        tests::fixture::{address, now, operator},
        tickets::{LIFETIME, TOKEN_BYTES, TicketToken},
    },
};

const CLIENT: &str = "100.64.0.7";

fn token(seed: u8) -> TicketToken {
    TicketToken::from_random([seed; TOKEN_BYTES])
}

fn ticket() -> PlaybackTicket {
    PlaybackTicket {
        operator: operator(),
        client: address(CLIENT),
        recording: RecordingId::from(Uuid::now_v7()),
        session: SessionId::from(Uuid::now_v7()),
        expires_at: now() + LIFETIME,
    }
}

/// `tickets` holding `ticket` under `token`, issued at [`now`].
fn holding(token: &TicketToken, ticket: &PlaybackTicket) -> Tickets {
    let tickets = Tickets::default();
    tickets.issue(token.hash(), ticket.clone(), now()).unwrap();
    tickets
}

#[test]
fn k4_1_a_playback_ticket_opens_once_with_what_it_was_issued_for() {
    let (token, issued) = (token(1), ticket());
    let tickets = holding(&token, &issued);
    let redeemed = tickets.redeem(token.expose(), address(CLIENT), now());
    assert_eq!(redeemed, Ok(issued));
    let again = tickets.redeem(token.expose(), address(CLIENT), now());
    assert_eq!(again, Err(Refused), "single use");
}

#[test]
fn k4_1_a_playback_ticket_lives_30_seconds_and_the_boundary_is_expired() {
    assert_eq!(LIFETIME, TimeDelta::seconds(30));
    let (token, issued) = (token(2), ticket());
    let last = now() + LIFETIME - TimeDelta::microseconds(1);
    let tickets = holding(&token, &issued);
    assert!(
        tickets
            .redeem(token.expose(), address(CLIENT), last)
            .is_ok()
    );
    let tickets = holding(&token, &issued);
    let at_expiry = tickets.redeem(token.expose(), address(CLIENT), now() + LIFETIME);
    assert_eq!(at_expiry, Err(Refused));
}

#[test]
fn k4_1_another_address_is_refused_and_spends_the_ticket() {
    let (token, issued) = (token(3), ticket());
    let tickets = holding(&token, &issued);
    let elsewhere = tickets.redeem(token.expose(), address("100.64.0.8"), now());
    assert_eq!(elsewhere, Err(Refused));
    let owner = tickets.redeem(token.expose(), address(CLIENT), now());
    assert_eq!(owner, Err(Refused), "removed when presented, valid or not");
    let expired = holding(&token, &issued);
    let late = expired.redeem(token.expose(), address(CLIENT), now() + LIFETIME);
    assert_eq!(late, Err(Refused));
    assert_eq!(expired.len(), 0, "an expired ticket is spent too");
}

#[test]
fn k4_1_only_the_exact_token_redeems() {
    let (token, issued) = (token(4), ticket());
    let tickets = holding(&token, &issued);
    let text = token.expose().to_owned();
    assert_eq!(text.len(), 43, "32 bytes in unpadded base64url");
    let near = [
        String::new(),
        format!("{text}="),
        text[..42].to_owned(),
        text.to_uppercase(),
        text.to_lowercase(),
        format!("{}A", &text[..42]),
        format!("{}B", &text[..42]),
        "a".repeat(4096),
    ];
    for presented in near.into_iter().filter(|presented| *presented != text) {
        let refused = tickets.redeem(&presented, address(CLIENT), now());
        assert_eq!(refused, Err(Refused), "{presented}");
    }
    assert_eq!(tickets.len(), 1, "a near miss spends nothing");
    assert!(tickets.redeem(&text, address(CLIENT), now()).is_ok());
}

#[test]
fn api_009_at_most_32_playback_tickets_are_pending_and_expired_ones_make_room() {
    assert_eq!(MAX_PENDING, 32);
    let tickets = Tickets::default();
    for seed in 0..32 {
        tickets.issue(token(seed).hash(), ticket(), now()).unwrap();
    }
    let full = tickets.issue(token(200).hash(), ticket(), now());
    assert!(full.is_err(), "the 33rd is refused");
    let later = now() + LIFETIME;
    tickets.issue(token(201).hash(), ticket(), later).unwrap();
    assert_eq!(tickets.len(), 1, "the expired 32 were dropped");
}

#[test]
fn api_009_a_withdrawn_ticket_never_opens() {
    let (token, issued) = (token(5), ticket());
    let tickets = holding(&token, &issued);
    tickets.withdraw(token.hash());
    let redeemed = tickets.redeem(token.expose(), address(CLIENT), now());
    assert_eq!(redeemed, Err(Refused));
}
