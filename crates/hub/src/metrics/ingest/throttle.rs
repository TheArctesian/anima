//! Which agent refusals with no valid credential reach the event log: at most one a minute
//! per client address, so a flood of guesses cannot fill the log (K8 Sections 3 and 6, CR-15).
//!
//! Pure: the time is a parameter, and the memory is bounded by [`ADDRESSES_PER_SPAN`]. Nothing
//! ticks: the count of refusals left out is written with the first refusal recorded a minute or
//! more after the first of them, so after the last refusal of a flood its count waits for the
//! next one. Past the budget a new address's refusal is counted, not written, for the rest of
//! the minute.

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, TimeDelta, Utc};

/// An address's refusal is written at most once in this span.
const SPAN: TimeDelta = TimeDelta::minutes(1);

/// The most addresses remembered in one span. Past it a refusal is counted, not written, and
/// the count is written once the span has passed.
pub(super) const ADDRESSES_PER_SPAN: usize = 64;

#[derive(Debug, Default)]
pub(super) struct Throttle {
    written: HashMap<IpAddr, DateTime<Utc>>,
    /// Refusals left out because the budget was spent, not yet counted in an event.
    unrecorded: usize,
    /// When the first of `unrecorded` was left out.
    since: Option<DateTime<Utc>>,
}

/// Whether `at` lies in the span that starts at `start`. A clock that went back ends the span.
fn within(start: DateTime<Utc>, at: DateTime<Utc>) -> bool {
    start <= at && at - start < SPAN
}

impl Throttle {
    /// Whether the refusal of `client` at `now` is written, and if so notes it: not when
    /// `client` was written in the last minute, nor when [`ADDRESSES_PER_SPAN`] addresses were.
    pub(super) fn admit(&mut self, client: IpAddr, now: DateTime<Utc>) -> bool {
        self.written.retain(|_, at| within(*at, now));
        if self.written.contains_key(&client) {
            return false;
        }
        if self.written.len() >= ADDRESSES_PER_SPAN {
            self.unrecorded += 1;
            self.since.get_or_insert(now);
            return false;
        }
        self.written.insert(client, now);
        true
    }

    /// How many refusals the budget left out, once a span has passed since the first of them;
    /// the count then starts again.
    pub(super) fn unrecorded(&mut self, now: DateTime<Utc>) -> Option<usize> {
        let since = self.since?;
        if within(since, now) {
            return None;
        }
        self.since = None;
        Some(std::mem::take(&mut self.unrecorded))
    }

    /// How many addresses are remembered.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.written.len()
    }
}
