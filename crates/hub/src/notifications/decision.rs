//! What to tell the operator after one heartbeat (NOT-001, NOT-005, NOT-006, Section 6.5):
//! pure, no clock and no I/O. Time is the beat's own `at`.

use chrono::{DateTime, Utc};

use super::model::{CheckState, ExpiryMark, ExpirySubject, NotificationKind, Observation};
use crate::status::Status;

#[cfg(test)]
mod tests;

/// One notification to queue. `days` is the whole days left, for the two expiry kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Notice {
    pub kind: NotificationKind,
    pub days: Option<u32>,
}

/// The state to keep and the notifications to queue.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Decision {
    pub state: CheckState,
    pub notices: Vec<Notice>,
}

/// Decides from one beat.
///
/// - Maintenance (the flag or the status) and Unknown change nothing and send nothing, expiry
///   included. A Down that began in maintenance is therefore sent only when a beat outside
///   maintenance is still Down.
/// - Down is sent when the status is Down and Down was not already announced. After that,
///   every `resend_every_beats` further Down beats send Still-down (0 never).
/// - Up is sent when Up or Degraded follows an announced Down. Degraded counts as back up and
///   never sends Down. Without an announced Down nothing is sent.
/// - Expiry: see `expiry`.
pub(super) fn decide(
    o: &Observation,
    state: &CheckState,
    resend_every_beats: u16,
    certificate_days: &[u16],
) -> Decision {
    let mut state = state.clone();
    let mut notices = Vec::new();
    if o.in_maintenance {
        return Decision { state, notices };
    }
    match o.status {
        Status::Maintenance | Status::Unknown => return Decision { state, notices },
        Status::Down => down(&mut state, resend_every_beats, &mut notices),
        Status::Up | Status::Degraded => back_up(o.status, &mut state, &mut notices),
    }
    expiry(o, certificate_days, &mut state.expiry_marks, &mut notices);
    Decision { state, notices }
}

fn plain(kind: NotificationKind) -> Notice {
    Notice { kind, days: None }
}

fn down(state: &mut CheckState, resend_every_beats: u16, notices: &mut Vec<Notice>) {
    if state.notified_status != Some(Status::Down) {
        state.notified_status = Some(Status::Down);
        state.down_beats = 0;
        notices.push(plain(NotificationKind::Down));
        return;
    }
    state.down_beats = state.down_beats.saturating_add(1);
    if resend_every_beats > 0 && state.down_beats >= u32::from(resend_every_beats) {
        state.down_beats = 0;
        notices.push(plain(NotificationKind::StillDown));
    }
}

fn back_up(status: Status, state: &mut CheckState, notices: &mut Vec<Notice>) {
    if state.notified_status == Some(Status::Down) {
        notices.push(plain(NotificationKind::Up));
    }
    state.notified_status = Some(status);
    state.down_beats = 0;
}

/// For the certificate and the domain apart: a threshold is crossed when the whole days left
/// are fewer than it. One notification when any crossed threshold has no mark; every crossed
/// threshold is then marked, so first sight at 5 days of 21, 14 and 7 sends one. A different
/// `notAfter` (a renewal) drops the old marks of that subject. No `notAfter` keeps them.
fn expiry(
    o: &Observation,
    certificate_days: &[u16],
    marks: &mut Vec<ExpiryMark>,
    notices: &mut Vec<Notice>,
) {
    let subjects = [
        (
            ExpirySubject::Certificate,
            o.certificate_not_after,
            NotificationKind::CertificateExpiry,
        ),
        (
            ExpirySubject::Domain,
            o.domain_not_after,
            NotificationKind::DomainExpiry,
        ),
    ];
    for (subject, not_after, kind) in subjects {
        let Some(not_after) = not_after else { continue };
        marks.retain(|m| m.subject != subject || m.not_after == not_after);
        let days_left = whole_days(not_after, o.at);
        let mut fresh = false;
        for threshold_days in certificate_days
            .iter()
            .copied()
            .filter(|t| days_left < u32::from(*t))
        {
            let mark = ExpiryMark {
                subject,
                threshold_days,
                not_after,
            };
            if !marks.contains(&mark) {
                marks.push(mark);
                fresh = true;
            }
        }
        if fresh {
            notices.push(Notice {
                kind,
                days: Some(days_left),
            });
        }
    }
}

/// Whole days from `at` to `not_after`; 0 once it has passed.
fn whole_days(not_after: DateTime<Utc>, at: DateTime<Utc>) -> u32 {
    u32::try_from((not_after - at).num_days()).unwrap_or(0)
}
