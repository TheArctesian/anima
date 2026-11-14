//! PRJ-041, LOG-007: the ticket and the cookie are 32 random bytes as base64url, the states are
//! derived from the cookie, each is kept only as a hash; and the words the operator reads name
//! no value.

use proptest::prelude::*;

use super::{app, app_created, exchanging, value, waiting};
use crate::engine::sources::registration::{
    attempt::{Failure, Phase, Snapshot},
    binding::{Minted, Step},
    words,
};

#[test]
fn prj_041_a_value_is_43_base64url_characters_and_never_repeats() {
    let (one, two) = (value(), value());
    assert_eq!(one.text().len(), 43);
    assert!(
        one.text()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    );
    assert_ne!(one.text(), two.text());
    assert!(one.hash().matches(one.text()));
    assert!(!one.hash().matches(two.text()));
}

proptest! {
    #[test]
    fn prj_041_only_the_value_itself_matches_its_hash(presented in ".{0,80}") {
        let minted = value();
        prop_assert_eq!(minted.hash().matches(&presented), presented == minted.text());
    }

    #[test]
    fn prj_041_a_changed_value_does_not_match(at in 0_usize..43, by in 1_u8..64) {
        let minted = value();
        let mut changed = minted.text().as_bytes().to_vec();
        changed[at] = changed[at].wrapping_add(by);
        let changed = String::from_utf8_lossy(&changed).into_owned();
        prop_assert!(!minted.hash().matches(&changed));
    }

    #[test]
    fn prj_041_a_state_is_the_same_for_one_browser_and_step_and_unrelated_otherwise(
        cookie in "\\PC{0,80}",
        other in "\\PC{0,80}",
    ) {
        let s1 = Minted::state(&cookie, Step::Create);
        let again = Minted::state(&cookie, Step::Create);
        let s2 = Minted::state(&cookie, Step::Install);
        prop_assert_eq!(s1.text(), again.text());
        prop_assert_eq!(s1.text().len(), 43);
        prop_assert!(s1.hash().matches(s1.text()));
        prop_assert!(!s1.hash().matches(s2.text()));
        if other != cookie {
            let theirs = Minted::state(&other, Step::Create);
            prop_assert_ne!(s1.text(), theirs.text());
        }
    }
}

fn registration() -> source_exchange::Registration {
    super::super::github(&crate::engine::sources::Registry::new(
        vec![provider_github::DECLARATION],
        Vec::new(),
    ))
    .unwrap()
}

#[test]
fn prj_039_the_install_page_shows_only_while_the_app_waits_for_it() {
    let registration = registration();
    let waiting_app = app_created().attempt.snapshot();
    assert_eq!(
        words::install_url(&waiting_app, &registration).as_deref(),
        Some("https://github.com/apps/anima-k3f9x2mq7b/installations/new")
    );
    assert_eq!(words::settings_url(&waiting_app, &registration), None);
    assert_eq!(
        words::install_url(&waiting().snapshot(), &registration),
        None
    );
}

#[test]
fn prj_039_the_delete_page_shows_when_an_app_may_exist() {
    let registration = registration();
    let ended = |failure: Option<Failure>, phase: Phase, with_app: bool| Snapshot {
        failure,
        phase,
        app: with_app.then(app),
        ..waiting().snapshot()
    };
    let apps = "https://github.com/settings/apps";
    let this_app = "https://github.com/settings/apps/anima-k3f9x2mq7b/advanced";
    let cases = [
        (ended(Some(Failure::Expired), Phase::Failed, false), None),
        (
            ended(Some(Failure::Expired), Phase::Failed, true),
            Some(this_app),
        ),
        (
            ended(Some(Failure::ExchangeRefused), Phase::Failed, false),
            Some(apps),
        ),
        (
            ended(Some(Failure::GithubUnreachable), Phase::Failed, false),
            Some(apps),
        ),
        (
            ended(Some(Failure::ReplyUnusable), Phase::Failed, false),
            Some(apps),
        ),
        (
            ended(Some(Failure::KeyRefused), Phase::Failed, true),
            Some(this_app),
        ),
        (ended(None, Phase::Cancelled, false), None),
        (ended(None, Phase::Cancelled, true), Some(this_app)),
        (ended(None, Phase::Connected, true), None),
    ];
    for (snapshot, expected) in cases {
        assert_eq!(
            words::settings_url(&snapshot, &registration).as_deref(),
            expected,
            "{:?} {:?}",
            snapshot.phase,
            snapshot.failure
        );
    }
}

#[test]
fn prj_039_the_words_follow_the_phase() {
    let mut connected = exchanging().attempt.snapshot();
    assert!(words::message(&connected).starts_with("Waiting for you on GitHub."));
    connected.phase = Phase::Connected;
    assert!(words::message(&connected).contains("Projects is off"));
    connected.consumer_on = true;
    assert_eq!(
        words::message(&connected),
        "GitHub is connected. The first sync has started."
    );
    let made = app_created().attempt.snapshot();
    assert_eq!(
        words::message(&made),
        "The App anima-k3f9x2mq7b is made. Press Install on GitHub."
    );
    assert_eq!(
        words::page_url("https://hub.example"),
        "https://hub.example/connect/github"
    );
}
