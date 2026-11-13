//! The files tickets (K4 Section 1): single use, bound to the client address, 30 s, bounded,
//! and kept apart from session tickets. Pure: the time is a parameter.

use chrono::TimeDelta;

use crate::{
    fleet::sessions::{
        Refused,
        files::tickets::{FilesTicket, Full, MAX_PENDING, Tickets},
        tests::fixture::{address, docker_host, now, operator},
        tickets::{LIFETIME, TicketToken},
    },
    ids::SessionId,
};

fn token(seed: u8) -> TicketToken {
    TicketToken::from_random([seed; 32])
}

fn ticket(client: &str) -> FilesTicket {
    FilesTicket {
        operator: operator(),
        client: address(client),
        session: SessionId::from(uuid::Uuid::now_v7()),
        device: docker_host(),
        expires_at: now() + LIFETIME,
    }
}

#[test]
fn api_009_a_ticket_opens_once() {
    let tickets = Tickets::default();
    let held = ticket("127.0.0.1");
    tickets.issue(token(1).hash(), held.clone(), now()).unwrap();
    let text = token(1);
    assert_eq!(
        tickets.redeem(text.expose(), address("127.0.0.1"), now()),
        Ok(held)
    );
    assert_eq!(
        tickets.redeem(text.expose(), address("127.0.0.1"), now()),
        Err(Refused)
    );
}

#[test]
fn api_009_another_address_spends_the_ticket() {
    let tickets = Tickets::default();
    tickets
        .issue(token(2).hash(), ticket("127.0.0.1"), now())
        .unwrap();
    let text = token(2);
    assert_eq!(
        tickets.redeem(text.expose(), address("127.0.0.2"), now()),
        Err(Refused)
    );
    assert_eq!(
        tickets.redeem(text.expose(), address("127.0.0.1"), now()),
        Err(Refused)
    );
}

#[test]
fn api_009_a_ticket_expires_after_30_seconds() {
    let tickets = Tickets::default();
    tickets
        .issue(token(3).hash(), ticket("127.0.0.1"), now())
        .unwrap();
    tickets
        .issue(token(4).hash(), ticket("127.0.0.1"), now())
        .unwrap();
    let last = now() + LIFETIME - TimeDelta::milliseconds(1);
    assert!(
        tickets
            .redeem(token(3).expose(), address("127.0.0.1"), last)
            .is_ok()
    );
    let expiry = now() + LIFETIME;
    assert_eq!(
        tickets.redeem(token(4).expose(), address("127.0.0.1"), expiry),
        Err(Refused)
    );
    assert_eq!(
        tickets.redeem("not a token", address("127.0.0.1"), now()),
        Err(Refused)
    );
}

#[test]
fn api_009_pending_tickets_are_bounded_and_expired_ones_make_room() {
    let tickets = Tickets::default();
    for seed in 0..MAX_PENDING {
        let seed = u8::try_from(seed).unwrap();
        tickets
            .issue(token(seed).hash(), ticket("127.0.0.1"), now())
            .unwrap();
    }
    assert_eq!(
        tickets.issue(token(200).hash(), ticket("127.0.0.1"), now()),
        Err(Full)
    );
    let later = now() + LIFETIME;
    tickets
        .issue(token(200).hash(), ticket("127.0.0.1"), later)
        .unwrap();
    assert_eq!(tickets.len(), 1, "the expired ones went");
}
