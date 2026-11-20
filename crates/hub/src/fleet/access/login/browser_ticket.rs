//! The open browser tickets of a hosted hub (hosted-mode.md HMD-061). The Anima.ing cloud asks
//! for one ticket for a browser of its account; the gateway trades it at once for the session
//! cookie of the hub. The hub keeps only the hash of a ticket, never the ticket (LOG-007). A
//! ticket works once and for a short time, and a restart of the hub drops them all.

use std::{
    collections::HashSet,
    sync::{Mutex, PoisonError},
};

use chrono::{DateTime, TimeDelta, Utc};
use sha2::{Digest, Sha256};

/// Random bytes in a ticket: 256 bits, so no throttle is needed to stop a guess.
pub const TICKET_BYTES: usize = 32;
/// How long a ticket works: the gateway trades it in the same step in which it asked for it.
pub const TICKET_LIFE: TimeDelta = TimeDelta::seconds(60);
/// The most open tickets. A ticket lives one minute, so only a flood reaches this; a new
/// ticket is then refused until some expire, and no memory grows without a bound.
const MOST_OPEN: usize = 1024;

/// The open tickets, by hash, with the time each ends.
#[derive(Default)]
pub struct BrowserTickets(Mutex<HashSet<([u8; 32], DateTime<Utc>)>>);

fn hash(ticket: &str) -> [u8; 32] {
    Sha256::digest(ticket.as_bytes()).into()
}

impl BrowserTickets {
    /// Keeps `ticket` until `now` plus [`TICKET_LIFE`] and returns that time; `None` when too
    /// many are open. The expired ones leave the book at the same time.
    pub fn open(&self, ticket: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut book = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        book.retain(|(_, ends)| now < *ends);
        if book.len() >= MOST_OPEN {
            return None;
        }
        let ends = now + TICKET_LIFE;
        book.insert((hash(ticket), ends));
        Some(ends)
    }

    /// Ends the ticket. `true` only for the first use of a ticket that is open and not expired.
    pub fn use_ticket(&self, ticket: &str, now: DateTime<Utc>) -> bool {
        let wanted = hash(ticket);
        let mut book = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let found = book.iter().find(|(kept, _)| *kept == wanted).copied();
        found.is_some_and(|entry| {
            book.remove(&entry);
            now < entry.1
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::UNIX_EPOCH + TimeDelta::seconds(seconds)
    }

    #[test]
    fn hmd_061_a_ticket_works_once_and_for_one_minute() {
        let tickets = BrowserTickets::default();
        assert_eq!(tickets.open("first", at(0)), Some(at(60)));
        assert!(!tickets.use_ticket("wrong", at(1)));
        assert!(tickets.use_ticket("first", at(1)));
        assert!(!tickets.use_ticket("first", at(2)), "a second use");
        tickets.open("late", at(0));
        assert!(!tickets.use_ticket("late", at(60)), "at its end");
        assert!(!tickets.use_ticket("late", at(61)), "it left the book");
    }

    #[test]
    fn expired_tickets_leave_the_book_and_a_flood_is_refused() {
        let tickets = BrowserTickets::default();
        for number in 0..MOST_OPEN {
            assert!(tickets.open(&format!("t{number}"), at(0)).is_some());
        }
        assert_eq!(tickets.open("one more", at(1)), None);
        assert_eq!(tickets.open("after they ended", at(60)), Some(at(120)));
        assert_eq!(tickets.0.lock().unwrap().len(), 1);
    }
}
