//! PRJ-039, PRJ-041: each step of the attempt and each refusal, with the time as a parameter.
//! A refused step leaves the attempt as it was.

use super::{Walked, app, app_created, at, at_github, exchanging, key, t0, value, waiting};
use crate::{
    engine::sources::registration::attempt::{
        Attempt, Exchange, Failure, Finished, Phase, Refusal,
    },
    ids::SourceAccountId,
};

fn phase(attempt: &Attempt) -> Phase {
    attempt.snapshot().phase
}

#[test]
fn prj_039_the_flow_walks_every_phase_and_drops_the_key_at_the_end() {
    let mut attempt = waiting();
    assert_eq!(phase(&attempt), Phase::WaitingForBrowser);
    let mut walked = app_created();
    assert_eq!(phase(&walked.attempt), Phase::AppCreated);
    assert!(walked.attempt.waiting_app().is_some(), "the key is held");
    let finish = walked.attempt.begin_finish().unwrap();
    assert_eq!(
        (finish.label.as_str(), finish.app_id, finish.key.as_str()),
        ("octo-person", 1_234_567, "KEY-MARKER"),
        "the default label is the owner's login"
    );
    assert_eq!(
        phase(&walked.attempt),
        Phase::AppCreated,
        "Finishing shows as APP_CREATED"
    );
    let account = SourceAccountId::from(uuid::Uuid::now_v7());
    let connected = Finished::Connected {
        account,
        consumer_on: true,
    };
    walked.attempt.finished(connected, at(10, 0));
    let shown = walked.attempt.snapshot();
    assert_eq!(
        (shown.phase, shown.account),
        (Phase::Connected, Some(account))
    );
    assert!(walked.attempt.waiting_app().is_none());
    assert!(walked.attempt.ended());
    assert!(attempt.issue_ticket(value().hash(), t0()).is_ok());
}

#[test]
fn prj_041_a_ticket_works_once_and_for_five_minutes() {
    let (ticket, cookie) = (value(), value());
    let mut attempt = waiting();
    let ends = attempt.issue_ticket(ticket.hash(), t0()).unwrap();
    assert_eq!(ends, at(5, 0));

    let mut late = waiting();
    late.issue_ticket(ticket.hash(), t0()).unwrap();
    assert_eq!(
        late.claim_ticket(ticket.text(), cookie.hash(), at(5, 0)),
        Err(Refusal::Ticket),
        "at five minutes"
    );
    assert_eq!(phase(&late), Phase::WaitingForBrowser, "unchanged");

    assert_eq!(
        attempt.claim_ticket(ticket.text(), cookie.hash(), at(4, 59)),
        Ok(())
    );
    assert_eq!(phase(&attempt), Phase::AtGithub);
    let second = value();
    assert_eq!(
        attempt.claim_ticket(ticket.text(), second.hash(), at(4, 59)),
        Err(Refusal::Ticket),
        "used once"
    );
    assert!(
        attempt.bound(Some(cookie.text())),
        "the first browser keeps it"
    );
    assert!(!attempt.bound(Some(second.text())));
}

#[test]
fn prj_041_a_new_ticket_ends_the_earlier_one_and_a_wrong_one_is_refused() {
    let (first, second, cookie) = (value(), value(), value());
    let mut attempt = waiting();
    attempt.issue_ticket(first.hash(), t0()).unwrap();
    attempt.issue_ticket(second.hash(), at(1, 0)).unwrap();
    for wrong in [first.text(), "", "x", &"a".repeat(5000)] {
        assert_eq!(
            attempt.claim_ticket(wrong, cookie.hash(), at(1, 0)),
            Err(Refusal::Ticket)
        );
    }
    assert_eq!(
        attempt.claim_ticket(second.text(), cookie.hash(), at(1, 0)),
        Ok(())
    );
}

#[test]
fn prj_041_a_signed_in_claim_ends_any_ticket() {
    let (ticket, cookie, other) = (value(), value(), value());
    let mut attempt = waiting();
    attempt.issue_ticket(ticket.hash(), t0()).unwrap();
    attempt.claim(cookie.hash()).unwrap();
    assert_eq!(
        attempt.claim_ticket(ticket.text(), other.hash(), t0()),
        Err(Refusal::Ticket)
    );
}

#[test]
fn prj_039_a_ticket_or_a_claim_is_refused_once_github_has_answered() {
    let mut walks: Vec<(&str, Walked)> = vec![("exchanging", exchanging()), ("app", app_created())];
    let mut cancelled = at_github();
    cancelled.attempt.cancel(t0()).unwrap();
    walks.push(("cancelled", cancelled));
    for (name, mut walked) in walks {
        let before = walked.attempt.snapshot();
        assert_eq!(
            walked.attempt.issue_ticket(value().hash(), t0()),
            Err(Refusal::Conflict),
            "{name}"
        );
        assert_eq!(
            walked.attempt.claim(value().hash()),
            Err(Refusal::Conflict),
            "{name}"
        );
        assert_eq!(walked.attempt.snapshot(), before, "{name}");
    }
}

#[test]
fn prj_039_the_return_from_github_needs_the_cookie_and_the_state_and_changes_nothing_otherwise() {
    let mut walked = at_github();
    let (cookie, state) = (
        walked.cookie.text().to_owned(),
        walked.state.text().to_owned(),
    );
    let other_browser = value();
    let cases: [(Option<&str>, Option<&str>, Refusal); 5] = [
        (None, Some(&state), Refusal::Unbound),
        (Some(other_browser.text()), Some(&state), Refusal::Unbound),
        (Some(&cookie), None, Refusal::State),
        (Some(&cookie), Some("wrong"), Refusal::State),
        (Some(&cookie), Some(other_browser.text()), Refusal::State),
    ];
    for (presented_cookie, presented_state, refusal) in cases {
        assert_eq!(
            walked
                .attempt
                .begin_exchange(presented_cookie, presented_state),
            Err(refusal)
        );
        assert_eq!(phase(&walked.attempt), Phase::AtGithub);
    }
    assert_eq!(
        walked.attempt.begin_exchange(Some(&cookie), Some(&state)),
        Ok(Exchange::Fresh),
        "no lockout"
    );
    assert_eq!(
        walked.attempt.begin_exchange(Some(&cookie), Some(&state)),
        Err(Refusal::Conflict),
        "the state is spent before the exchange"
    );
}

#[test]
fn prj_039_a_return_after_the_app_is_made_converts_nothing() {
    let mut walked = app_created();
    let cookie = walked.cookie.text().to_owned();
    assert_eq!(
        walked
            .attempt
            .begin_exchange(Some(&cookie), Some("anything")),
        Ok(Exchange::Made)
    );
    assert_eq!(
        walked.attempt.begin_exchange(None, None),
        Err(Refusal::Unbound)
    );
    let s2 = value();
    assert_eq!(
        walked.attempt.send_to_install(Some(&cookie), s2.hash()),
        Ok(())
    );
    assert!(
        walked
            .attempt
            .back_from_install(Some(&cookie), Some(s2.text()))
    );
    assert!(
        walked.attempt.back_from_install(Some(&cookie), None),
        "GitHub may send no state"
    );
    assert!(
        !walked
            .attempt
            .back_from_install(Some(&cookie), Some(walked.state.text()))
    );
    assert!(!walked.attempt.back_from_install(None, Some(s2.text())));
}

#[test]
fn prj_039_the_form_needs_the_bound_browser() {
    let mut walked = at_github();
    let s1 = value();
    assert_eq!(
        walked
            .attempt
            .send_to_github(Some(value().text()), s1.hash()),
        Err(Refusal::Unbound)
    );
    assert_eq!(
        walked.attempt.send_to_github(None, s1.hash()),
        Err(Refusal::Unbound)
    );
    let cookie = walked.cookie.text().to_owned();
    walked
        .attempt
        .send_to_github(Some(&cookie), s1.hash())
        .unwrap();
    assert_eq!(
        walked
            .attempt
            .begin_exchange(Some(&cookie), Some(walked.state.text())),
        Err(Refusal::State),
        "a new form replaces the earlier state"
    );
    let mut fresh = waiting();
    assert_eq!(fresh.send_to_github(None, s1.hash()), Err(Refusal::Unbound));
}

#[test]
fn prj_039_each_open_phase_expires_at_55_minutes_and_drops_the_key() {
    let walks = [
        ("waiting", waiting()),
        ("at github", at_github().attempt),
        ("exchanging", exchanging().attempt),
        ("app created", app_created().attempt),
    ];
    for (name, mut attempt) in walks {
        let before = phase(&attempt);
        assert!(!attempt.expire(at(54, 59)), "{name}");
        assert_eq!(phase(&attempt), before, "{name}");
        assert!(attempt.expire(at(55, 0)), "{name}");
        let shown = attempt.snapshot();
        assert_eq!(
            (shown.phase, shown.failure),
            (Phase::Failed, Some(Failure::Expired)),
            "{name}"
        );
        assert!(attempt.waiting_app().is_none(), "{name}: no key");
        assert!(!attempt.expire(at(56, 0)), "{name}: once");
    }
    let mut walked = app_created();
    assert!(walked.attempt.begin_finish().is_ok());
    assert!(!walked.attempt.expire(at(56, 0)), "the store decides");
    walked.attempt.finished(Finished::Again, at(56, 0));
    assert_eq!(walked.attempt.snapshot().failure, Some(Failure::Expired));
}

#[test]
fn prj_039_only_one_caller_stores_and_a_failed_store_waits_for_the_next_poll() {
    let mut walked = app_created();
    assert!(walked.attempt.begin_finish().is_ok());
    assert!(matches!(
        walked.attempt.begin_finish(),
        Err(Refusal::Conflict)
    ));
    assert!(
        walked.attempt.waiting_app().is_none(),
        "no poll while storing"
    );
    walked.attempt.finished(Finished::Again, at(1, 0));
    assert!(walked.attempt.waiting_app().is_some(), "the key is back");
    assert!(walked.attempt.begin_finish().is_ok());
    walked
        .attempt
        .finished(Finished::Failed(Failure::KeyRefused), at(2, 0));
    let shown = walked.attempt.snapshot();
    assert_eq!(
        (shown.phase, shown.failure),
        (Phase::Failed, Some(Failure::KeyRefused))
    );
    assert!(walked.attempt.waiting_app().is_none());
}

#[test]
fn prj_039_cancel_drops_the_key_and_is_refused_once_connected() {
    for mut walked in [at_github(), exchanging(), app_created()] {
        assert_eq!(walked.attempt.cancel(at(1, 0)), Ok(()));
        assert_eq!(phase(&walked.attempt), Phase::Cancelled);
        assert!(walked.attempt.waiting_app().is_none());
        assert_eq!(walked.attempt.cancel(at(2, 0)), Ok(()), "again: no change");
    }
    let mut storing = app_created();
    storing.attempt.begin_finish().unwrap();
    assert_eq!(storing.attempt.cancel(at(1, 0)), Err(Refusal::Conflict));
    let account = SourceAccountId::from(uuid::Uuid::now_v7());
    let connected = Finished::Connected {
        account,
        consumer_on: false,
    };
    storing.attempt.finished(connected, at(1, 0));
    assert_eq!(storing.attempt.cancel(at(2, 0)), Err(Refusal::Conflict));
}

#[test]
fn prj_039_an_app_made_after_a_cancel_is_named_but_its_key_is_dropped() {
    let mut walked = exchanging();
    walked.attempt.cancel(at(1, 0)).unwrap();
    assert_eq!(
        walked.attempt.created(app(), key(), value().hash()),
        Err(Refusal::Conflict)
    );
    let shown = walked.attempt.snapshot();
    assert_eq!((shown.phase, shown.app), (Phase::Cancelled, Some(app())));
    assert!(walked.attempt.waiting_app().is_none());
}

#[test]
fn prj_039_a_failed_exchange_ends_the_attempt_with_its_failure() {
    let mut walked = exchanging();
    walked
        .attempt
        .exchange_failed(Failure::ExchangeRefused, None, at(1, 0));
    let shown = walked.attempt.snapshot();
    assert_eq!(
        (shown.phase, shown.failure),
        (Phase::Failed, Some(Failure::ExchangeRefused))
    );
    assert!(walked.attempt.readable(at(10, 59)));
    assert!(
        !walked.attempt.readable(at(11, 0)),
        "ten minutes after the end"
    );
}
