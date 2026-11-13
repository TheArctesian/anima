//! The decision core as a table of beats (NOT-001, NOT-005, NOT-006, Section 6.5).

use chrono::{DateTime, TimeDelta, Utc};

use super::{Notice, decide};
use crate::{
    fleet::notifications::{
        CheckState, ExpiryMark, ExpirySubject, NotificationKind as Kind, Observation,
    },
    ids::{CheckId, DeviceId},
    status::Status::{self, Degraded, Down, Maintenance, Unknown, Up},
};

const DAYS: [u16; 3] = [21, 14, 7];

fn t0() -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000, 0).unwrap()
}

fn beat(status: Status) -> Observation {
    Observation {
        check: CheckId::from(uuid::Uuid::nil()),
        check_name: "ping".into(),
        check_type: "ping",
        device: (DeviceId::from(uuid::Uuid::nil()), "pve-01".into()),
        site: "home".into(),
        previous: Up,
        status,
        in_maintenance: false,
        at: t0(),
        message: None,
        certificate_not_after: None,
        domain_not_after: None,
    }
}

fn kinds(notices: &[Notice]) -> Vec<Kind> {
    notices.iter().map(|n| n.kind).collect()
}

/// Runs beats in order and returns the kinds sent at each.
fn run(beats: &[Observation], resend: u16) -> Vec<Vec<Kind>> {
    let mut state = CheckState::default();
    beats
        .iter()
        .map(|o| {
            let d = decide(o, &state, resend, &DAYS);
            state = d.state;
            kinds(&d.notices)
        })
        .collect()
}

fn statuses(list: &[Status]) -> Vec<Observation> {
    list.iter().map(|s| beat(*s)).collect()
}

fn maintained(status: Status) -> Observation {
    Observation {
        in_maintenance: true,
        ..beat(status)
    }
}

#[test]
fn not_001_down_is_sent_once_and_up_once_after_it() {
    let sent = run(&statuses(&[Up, Down, Down, Down, Up, Up]), 0);
    assert_eq!(
        sent,
        [
            vec![],
            vec![Kind::Down],
            vec![],
            vec![],
            vec![Kind::Up],
            vec![]
        ]
    );
}

#[test]
fn not_001_degraded_after_down_is_back_up_and_never_sends_down() {
    let sent = run(&statuses(&[Degraded, Down, Degraded, Down]), 0);
    assert_eq!(
        sent,
        [vec![], vec![Kind::Down], vec![Kind::Up], vec![Kind::Down]]
    );
}

#[test]
fn not_001_unknown_changes_nothing() {
    let sent = run(&statuses(&[Down, Unknown, Down, Unknown, Up]), 0);
    assert_eq!(
        sent,
        [vec![Kind::Down], vec![], vec![], vec![], vec![Kind::Up]]
    );
}

#[test]
fn mw_6_5_nothing_is_sent_in_maintenance_and_no_up_follows_an_unsent_down() {
    let beats = [
        maintained(Down),
        maintained(Down),
        beat(Maintenance),
        beat(Up),
    ];
    assert_eq!(run(&beats, 0), [vec![], vec![], vec![], vec![]]);
}

#[test]
fn mw_6_5_a_down_that_began_in_maintenance_is_sent_when_the_next_beat_is_still_down() {
    let beats = [maintained(Down), beat(Down), beat(Down)];
    assert_eq!(run(&beats, 0), [vec![], vec![Kind::Down], vec![]]);
}

#[test]
fn mw_6_5_up_after_maintenance_is_sent_only_when_down_was_sent_before() {
    let beats = [beat(Down), maintained(Down), maintained(Up), beat(Up)];
    assert_eq!(
        run(&beats, 0),
        [vec![Kind::Down], vec![], vec![], vec![Kind::Up]]
    );
}

#[test]
fn mw_6_5_maintenance_beats_do_not_count_toward_resend() {
    let beats = [beat(Down), maintained(Down), maintained(Down), beat(Down)];
    assert_eq!(run(&beats, 2), [vec![Kind::Down], vec![], vec![], vec![]]);
}

#[test]
fn mw_6_5_maintenance_holds_back_expiry() {
    let mut o = maintained(Up);
    o.certificate_not_after = Some(t0() + TimeDelta::days(3));
    let d = decide(&o, &CheckState::default(), 0, &DAYS);
    assert_eq!(d.notices, Vec::new());
    assert_eq!(d.state, CheckState::default());
}

#[test]
fn not_005_still_down_is_sent_at_n_and_2n_down_beats_after_down() {
    let mut beats = statuses(&[Down; 7]);
    beats.push(beat(Up));
    let sent = run(&beats, 3);
    let still = vec![Kind::StillDown];
    assert_eq!(
        sent,
        [
            vec![Kind::Down],
            vec![],
            vec![],
            still.clone(),
            vec![],
            vec![],
            still,
            vec![Kind::Up]
        ]
    );
}

#[test]
fn not_005_zero_never_resends() {
    let sent = run(&statuses(&[Down; 50]), 0);
    assert_eq!(sent.iter().flatten().count(), 1);
}

#[test]
fn not_005_resend_of_one_sends_still_down_on_every_further_beat() {
    let sent = run(&statuses(&[Down, Down, Down]), 1);
    assert_eq!(
        sent,
        [
            vec![Kind::Down],
            vec![Kind::StillDown],
            vec![Kind::StillDown]
        ]
    );
}

fn with_cert(days: i64, hours: i64) -> Observation {
    Observation {
        certificate_not_after: Some(t0() + TimeDelta::days(days) + TimeDelta::hours(hours)),
        ..beat(Up)
    }
}

fn at_days_before(mut o: Observation, days: i64) -> Observation {
    o.at = t0() + TimeDelta::days(days);
    o
}

/// The not-after of `with_cert(30, 0)`, seen from `day` days after `t0`.
fn expiring(day: i64) -> Observation {
    at_days_before(with_cert(30, 0), day)
}

fn run_expiry(beats: &[Observation]) -> Vec<Vec<Notice>> {
    let mut state = CheckState::default();
    beats
        .iter()
        .map(|o| {
            let d = decide(o, &state, 0, &DAYS);
            state = d.state;
            d.notices
        })
        .collect()
}

fn cert(days: u32) -> Vec<Notice> {
    vec![Notice {
        kind: Kind::CertificateExpiry,
        days: Some(days),
    }]
}

#[test]
fn not_006_one_notification_per_threshold_as_days_run_down() {
    // 30 days out: nothing. Day 10 (20 left): 21. Day 11: nothing. Day 17 (13 left): 14.
    // Day 24 (6 left): 7. Day 25: nothing.
    let beats = [
        expiring(0),
        expiring(10),
        expiring(11),
        expiring(17),
        expiring(24),
        expiring(25),
    ];
    assert_eq!(
        run_expiry(&beats),
        [vec![], cert(20), vec![], cert(13), cert(6), vec![]]
    );
}

#[test]
fn not_006_a_threshold_is_crossed_only_when_fewer_whole_days_are_left() {
    let exactly_seven = run_expiry(&[at_days_before(with_cert(30, 0), 23), expiring(24)]);
    assert_eq!(exactly_seven, [cert(7), cert(6)]);
}

#[test]
fn not_006_first_sight_at_five_days_sends_one_and_marks_every_crossed_threshold() {
    let o = with_cert(5, 3);
    let d = decide(&o, &CheckState::default(), 0, &DAYS);
    assert_eq!(d.notices, cert(5));
    assert_eq!(d.state.expiry_marks.len(), 3);
    let again = decide(&o, &d.state, 0, &DAYS);
    assert_eq!(again.notices, Vec::new());
}

#[test]
fn not_006_a_renewed_certificate_drops_the_old_marks_and_starts_again() {
    let first = with_cert(5, 0);
    let d = decide(&first, &CheckState::default(), 0, &DAYS);
    let renewed = with_cert(90, 0);
    let quiet = decide(&renewed, &d.state, 0, &DAYS);
    assert_eq!(quiet.notices, Vec::new());
    assert_eq!(quiet.state.expiry_marks, Vec::new());
    let later = at_days_before(renewed, 80);
    let again = decide(&later, &quiet.state, 0, &DAYS);
    assert_eq!(again.notices, cert(10));
}

#[test]
fn not_006_certificate_and_domain_are_marked_and_sent_apart() {
    let o = Observation {
        domain_not_after: Some(t0() + TimeDelta::days(20)),
        ..with_cert(5, 0)
    };
    let d = decide(&o, &CheckState::default(), 0, &DAYS);
    assert_eq!(
        kinds(&d.notices),
        [Kind::CertificateExpiry, Kind::DomainExpiry]
    );
    let subjects: Vec<_> = d.state.expiry_marks.iter().map(|m| m.subject).collect();
    assert_eq!(
        subjects
            .iter()
            .filter(|s| **s == ExpirySubject::Domain)
            .count(),
        1
    );
    // The domain is marked at 21 only; a certificate renewal leaves that mark alone.
    let renewed = Observation {
        certificate_not_after: Some(t0() + TimeDelta::days(100)),
        ..o
    };
    let after = decide(&renewed, &d.state, 0, &DAYS);
    assert_eq!(after.notices, Vec::new());
    assert!(
        after
            .state
            .expiry_marks
            .iter()
            .all(|m: &ExpiryMark| m.subject == ExpirySubject::Domain)
    );
}

#[test]
fn not_006_a_beat_without_a_not_after_keeps_the_marks() {
    let d = decide(&with_cert(5, 0), &CheckState::default(), 0, &DAYS);
    let silent = decide(&beat(Up), &d.state, 0, &DAYS);
    assert_eq!(silent.state.expiry_marks, d.state.expiry_marks);
}

#[test]
fn not_006_an_expired_certificate_reports_zero_days() {
    let d = decide(&with_cert(-2, 0), &CheckState::default(), 0, &DAYS);
    assert_eq!(d.notices, cert(0));
}

#[test]
fn not_006_expiry_is_sent_while_down_as_well() {
    let o = Observation {
        certificate_not_after: Some(t0() + TimeDelta::days(3)),
        ..beat(Down)
    };
    let d = decide(&o, &CheckState::default(), 0, &DAYS);
    assert_eq!(kinds(&d.notices), [Kind::Down, Kind::CertificateExpiry]);
}
