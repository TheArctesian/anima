//! The one hub that the settings are made for (K10 Section 6.4.2).

use super::{OF_ANIMA, SET_ON_AT, World, item, pair};
use crate::{
    Pair, PairDecision, PairFile, PairRecord, Refusal, pair_decided, request,
    work::{Accepted, checked},
};

fn other() -> Pair {
    Pair::of(
        "https://other.example:443",
        "0192f000-0000-7000-8000-000000000001",
    )
    .expect("a pair")
}

#[test]
fn k10_6_4_2_the_origin_is_the_scheme_the_host_in_lower_case_and_the_port_always_written() {
    let device = "0192f000-0000-7000-8000-000000000001";
    for origin in [
        "https://hub.example:443",
        "http://10.0.0.5:8080",
        "https://[fd7a::1]:8443",
        "https://[2001:db8::a:1]:443",
        "http://hub-1.lan:8080",
    ] {
        assert!(Pair::of(origin, device).is_some(), "{origin}");
    }
    for origin in [
        "https://hub.example",
        "https://Hub.example:443",
        "hub.example:443",
        "https://hub.example:443/",
        "https://hub.example:0443",
        "https://hub.example:70000",
        "https://user@hub.example:443",
        "unix:/run/anima/hub.sock",
        "https://[FD7A::1]:443",
        "http://a..b:1",
        "http://.a:1",
        "http://a-.b:1",
        "https://[fd7a:0::1]:443",
        "https://[fd7a:0:0:0:0:0:0:1]:443",
        "https://[::ffff:10.0.0.5]:443",
        "https://[::ffff:a00:5]:443",
        "http://010.0.0.5:8080",
        "http://10.0.5:8080",
        "http://10.0.0.5.:8080",
        "http://hub.123:8080",
        "unix:///run/anima/hub.sock",
        "",
    ] {
        assert!(Pair::of(origin, device).is_none(), "{origin}");
    }
    assert!(
        Pair::of(
            "https://hub.example:443",
            "0192F000-0000-7000-8000-000000000001"
        )
        .is_none()
    );
    assert!(Pair::of("https://hub.example:443", "device").is_none());
}

#[test]
fn k10_6_4_2_the_first_pair_is_recorded_and_the_same_pair_is_accepted() {
    let PairDecision::First(record) = pair_decided(PairFile::Missing, SET_ON_AT, &pair()) else {
        panic!("the first pair is recorded");
    };
    assert_eq!(record.first, pair());
    assert!(!record.different_seen);
    assert_eq!(
        pair_decided(PairFile::Read(&record), SET_ON_AT, &pair()),
        PairDecision::Recorded
    );
    assert_eq!(PairRecord::read(&record.written()), Some(record));
}

#[test]
fn k10_6_4_2_a_different_pair_refuses_each_item_until_the_setting_is_set_on_again() {
    let PairDecision::First(record) = pair_decided(PairFile::Missing, SET_ON_AT, &pair()) else {
        panic!("the first pair is recorded");
    };
    let PairDecision::Different(Some(marked)) =
        pair_decided(PairFile::Read(&record), SET_ON_AT, &other())
    else {
        panic!("a different pair is refused");
    };
    assert!(marked.different_seen);
    assert_eq!(marked.first, pair());
    // Also the recorded pair is then refused.
    assert_eq!(
        pair_decided(PairFile::Read(&marked), SET_ON_AT, &pair()),
        PairDecision::Different(None)
    );
    // The administrator sets "Manage applications" on again: the record is not valid.
    let again = pair_decided(PairFile::Read(&marked), SET_ON_AT + 60, &other());
    assert!(matches!(again, PairDecision::First(record) if record.first == other()));
}

#[test]
fn k10_6_4_2_a_request_with_a_different_pair_gets_applications_off() {
    let mut world = World::new();
    let stop = item(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
    );
    let stop = request::read(&stop).expect("an item");
    let verdict = checked(&stop, &pair(), &world.facts());
    assert!(matches!(verdict.outcome, Ok(Accepted::Ready(_))));
    assert_eq!(verdict.record, None);

    let verdict = checked(&stop, &other(), &world.facts());
    assert_eq!(verdict.outcome, Err(Refusal::ApplicationsOff));
    assert!(verdict.record.is_some_and(|record| record.different_seen));

    world.pair = None;
    let verdict = checked(&stop, &other(), &world.facts());
    assert!(verdict.outcome.is_ok());
    assert!(verdict.record.is_some_and(|record| record.first == other()));
}

#[test]
fn k10_6_4_2_a_record_that_cannot_be_read_refuses_each_item_and_is_never_no_record() {
    assert_eq!(
        pair_decided(PairFile::Unreadable, SET_ON_AT, &pair()),
        PairDecision::Different(None)
    );
    let world = World::new();
    let stop = item(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
    );
    let stop = request::read(&stop).expect("an item");
    let mut facts = world.facts();
    facts.pair = PairFile::Unreadable;
    for sent in [pair(), other()] {
        let verdict = checked(&stop, &sent, &facts);
        assert_eq!(verdict.outcome, Err(Refusal::ApplicationsOff));
        assert_eq!(verdict.record, None, "the file is not written again");
    }
}

#[test]
fn k10_5_each_type_is_refused_with_applications_off_while_the_setting_is_off() {
    let mut world = World::new();
    world.settings.managed = None;
    let application = super::APPLICATION;
    for (kind, parameters) in [
        (
            "container_action",
            format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
        ),
        (
            "container_follow",
            format!(r#"{{"container":"{OF_ANIMA}","tail":10}}"#),
        ),
        (
            "application_apply",
            format!(
                r#"{{"application":"{application}","fileSha256":"{OF_ANIMA}","environmentSha256":null,"pull":true,"keep":0}}"#
            ),
        ),
        (
            "adopted_apply",
            r#"{"project":"tracks","action":"stop","pull":false}"#.to_owned(),
        ),
        (
            "application_remove",
            format!(r#"{{"application":"{application}","volumes":false,"images":false}}"#),
        ),
        (
            "application_back",
            format!(r#"{{"application":"{application}"}}"#),
        ),
        ("docker_prune", r#"{"what":["containers"]}"#.to_owned()),
    ] {
        let request = request::read(&item(kind, &parameters)).expect("an item");
        let verdict = checked(&request, &pair(), &world.facts());
        assert_eq!(verdict.outcome, Err(Refusal::ApplicationsOff), "{kind}");
        assert_eq!(verdict.record, None);
    }
}

#[test]
fn k10_6_4_2_a_record_that_the_program_did_not_write_is_not_read() {
    for text in [
        "",
        "{}",
        r#"{"setOnAt":1,"origin":"https://hub.example","device":"0192f000-0000-7000-8000-000000000001","differentSeen":false}"#,
        r#"{"setOnAt":1,"origin":"https://hub.example:443","device":"0192f000-0000-7000-8000-000000000001","differentSeen":false,"more":1}"#,
    ] {
        assert_eq!(PairRecord::read(text), None, "{text}");
    }
}
