//! The pure parts of Connect GitHub: the attempt machine with the time as a parameter, the one
//! slot that holds it, the three values and the words (K-E1 Section 7a, PRJ-039, PRJ-041).

mod ending;
mod machine;
mod slot;
mod values;

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use ring::rand::SystemRandom;
use source_exchange::Owner;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    attempt::{App, Attempt, Phase, Request, Starter},
    binding::Minted,
};
use crate::{
    fleet::events::{Actor, ActorKind, ClientAddr, Operator, TokenKind},
    ids::OperatorId,
};

fn phase(attempt: &Attempt) -> Phase {
    attempt.snapshot().phase
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

fn at(minutes: i64, seconds: i64) -> DateTime<Utc> {
    t0() + TimeDelta::minutes(minutes) + TimeDelta::seconds(seconds)
}

fn by(kind: ActorKind) -> Starter {
    Starter {
        operator: Operator {
            id: OperatorId::from(Uuid::nil()),
            name: "operator".to_owned(),
            actor: Actor::request(kind, TokenKind::Login, None),
        },
        client: ClientAddr("100.64.0.7".parse().unwrap()),
    }
}

fn request(label: Option<&str>) -> Request {
    Request {
        label: label.map(str::to_owned),
        organisation: None,
        name: "anima-k3f9x2mq7b".to_owned(),
        by: by(ActorKind::User),
    }
}

fn value() -> Minted {
    Minted::new(&SystemRandom::new()).unwrap()
}

fn app() -> App {
    App {
        id: 1_234_567,
        slug: "anima-k3f9x2mq7b".to_owned(),
        owner: Owner {
            id: 9001,
            login: "octo-person".to_owned(),
            organisation: false,
        },
    }
}

fn key() -> Zeroizing<String> {
    Zeroizing::new("KEY-MARKER".to_owned())
}

/// An attempt walked to one phase, with the browser's cookie and the last state it was given.
struct Walked {
    attempt: Attempt,
    cookie: Minted,
    state: Minted,
}

fn waiting() -> Attempt {
    Attempt::start(Uuid::now_v7(), request(None), t0())
}

/// Bound to a browser by a ticket, the form to GitHub served with a state.
fn at_github() -> Walked {
    let mut attempt = waiting();
    let (ticket, cookie, state) = (value(), value(), value());
    attempt.issue_ticket(ticket.hash(), t0()).unwrap();
    attempt
        .claim_ticket(ticket.text(), cookie.hash(), t0())
        .unwrap();
    attempt
        .send_to_github(Some(cookie.text()), state.hash())
        .unwrap();
    Walked {
        attempt,
        cookie,
        state,
    }
}

/// Back from GitHub, the code being converted.
fn exchanging() -> Walked {
    let mut walked = at_github();
    let (cookie, state) = (walked.cookie.text(), walked.state.text());
    walked
        .attempt
        .begin_exchange(Some(cookie), Some(state))
        .unwrap();
    walked
}

/// The App made, waiting for Install.
fn app_created() -> Walked {
    let mut walked = exchanging();
    let s2 = value();
    walked.attempt.created(app(), key(), s2.hash()).unwrap();
    Walked {
        state: s2,
        ..walked
    }
}
