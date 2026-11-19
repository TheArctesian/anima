//! The sweep of each minute (K-E6 Sections 3 and 4; REM-002, REM-006): the time rules, and the
//! place rules from the location samples that arrived since the last sweep. The caller gives
//! `now`. A hub that was stopped fires a time rule that it missed one time, and a late import
//! of old samples makes no arrival in the past.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::{
    Error,
    model::{Occurrence, PlaceChange, Trigger},
    occurrences::{self, Extensions},
    place_state::place_changes,
    sending::{WasKnown, fire, notice},
    store,
};
use crate::{
    engine::{locations, places, sources},
    fleet::{credentials::operational, events},
};

/// The cursor of the location samples in `reminders.cursor`.
const SAMPLES: &str = "locations";
/// The cursor of the event log in `reminders.cursor`.
const EVENTS: &str = "events";
/// The most samples one sweep reads; the next sweep reads on.
const BATCH: i64 = 5000;
/// How far back the first sweep reads: older samples fire no place rule (R-145).
const FIRST_REACH: TimeDelta = TimeDelta::minutes(10);

/// What one sweep did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swept {
    /// Firings that the sweep stored, pending or suppressed.
    pub firings: u64,
    /// Places whose state changed.
    pub place_changes: usize,
}

/// Evaluates each enabled rule whose trigger is `wanted`, with the place changes of this sweep,
/// and gives how many firings were stored.
async fn fire_rules(
    conn: &mut PgConnection,
    wanted: impl Fn(&Trigger) -> bool,
    changes: &[PlaceChange],
    now: DateTime<Utc>,
) -> Result<u64, Error> {
    let known = WasKnown::read(conn, now).await?;
    let mut firings = 0;
    for stored in store::enabled_rules(conn).await? {
        if wanted(&stored.rule.trigger) {
            firings += fire(conn, (&stored.rule, &known), (&[], changes), now).await?;
        }
    }
    Ok(firings)
}

/// Evaluates each enabled time rule at `now`, in one transaction.
async fn times(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    let mut tx = db.begin().await?;
    store::lock_sweep(&mut tx).await?;
    let firings = fire_rules(
        &mut tx,
        |trigger| matches!(trigger, Trigger::Time(_)),
        &[],
        now,
    )
    .await?;
    tx.commit().await?;
    Ok(firings)
}

/// The samples of a read without a last group that the batch may have cut: a full batch can
/// end inside a group of samples of one instant, and the next sweep reads that group as a
/// whole.
fn whole_instants(mut samples: Vec<locations::Sample>) -> Vec<locations::Sample> {
    let full = i64::try_from(samples.len()).is_ok_and(|read| read == BATCH);
    let last = samples.last().map(|sample| sample.received_at);
    if full && last != samples.first().map(|sample| sample.received_at) {
        samples.retain(|sample| Some(sample.received_at) != last);
    }
    samples
}

/// Reads the samples that arrived since the cursor, changes the place states, evaluates each
/// enabled place rule and moves the cursor, in one transaction.
async fn arrivals(db: &PgPool, seal: &locations::Seal, now: DateTime<Utc>) -> Result<Swept, Error> {
    let known_places = places::list(db).await?;
    let mut tx = db.begin().await?;
    store::lock_sweep(&mut tx).await?;
    let after = store::cursor(&mut tx, SAMPLES).await?;
    let after = after.unwrap_or(now - FIRST_REACH);
    let read = locations::received(&mut tx, seal, (after, now), BATCH).await?;
    let samples = whole_instants(read);
    let Some(newest) = samples.iter().map(|sample| sample.received_at).max() else {
        return Ok(Swept::default());
    };
    let states = store::place_states(&mut tx).await?;
    let changes = place_changes(&states, &samples, &known_places);
    for change in &changes {
        store::put_place_state(&mut tx, change).await?;
    }
    let is_place = |trigger: &Trigger| matches!(trigger, Trigger::Place { .. });
    let firings = fire_rules(&mut tx, is_place, &changes, now).await?;
    store::set_cursor(&mut tx, SAMPLES, newest).await?;
    tx.commit().await?;
    Ok(Swept {
        firings,
        place_changes: changes.len(),
    })
}

/// What a sweep reads besides the database: the sealing keys of the positions, the vault-free
/// reads of the engine, and the extensions that give occurrences.
#[derive(Clone)]
pub struct Reading {
    pub seal: locations::Seal,
    pub extensions: Extensions,
    /// The sources of the hub, for the health of their accounts.
    pub sources: sources::Registry,
}

/// What has no moment of storage: the operational keys that end or need a rotation, and the
/// expiries and measures of each extension that is on.
async fn pulled(
    db: &PgPool,
    extensions: &Extensions,
    now: DateTime<Utc>,
) -> Result<Vec<Occurrence>, Error> {
    let keys = operational::keys(db, now).await?;
    let mut new = occurrences::of_keys(&keys);
    let on = crate::engine::extensions::on_keys(db).await?;
    let read = extensions
        .iter()
        .filter(|extension| on.iter().any(|key| key.as_str() == extension.name()));
    for extension in read {
        new.extend(extension.expiries(db, now).await?);
        new.extend(extension.measures(db, now).await?);
    }
    Ok(new)
}

/// Reads what the engine follows: the new events of the event log since the cursor and the
/// changes of health of the source accounts, with what [`pulled`] gives. Then it evaluates the
/// rules that wait for occurrences, in one transaction with the cursor.
async fn given(db: &PgPool, reading: &Reading, now: DateTime<Utc>) -> Result<(), Error> {
    let seen = occurrences::seen(sources::healths(db, &reading.sources, now).await?);
    let mut new = pulled(db, &reading.extensions, now).await?;
    let mut tx = db.begin().await?;
    store::lock_sweep(&mut tx).await?;
    // The first sweep starts at now: an old event fires nothing.
    let after = store::cursor(&mut tx, EVENTS).await?.unwrap_or(now);
    let events = events::after(&mut tx, (after, now), BATCH).await?;
    let newest = events.last().map_or(now, |event| event.at);
    new.extend(events.iter().filter_map(occurrences::of_event));
    let before = store::states(&mut tx).await?;
    new.extend(occurrences::of_health(&before, &seen, now));
    for account in &seen {
        store::remember_state(&mut tx, (&account.account, account.state), now).await?;
    }
    notice(&mut tx, &new, now).await?;
    store::set_cursor(&mut tx, EVENTS, newest.max(after)).await?;
    tx.commit().await?;
    Ok(())
}

/// One sweep at `now`: the time rules, then what the engine and the extensions give, then
/// the place rules. The time rules are stored also
/// when the samples cannot be read (no sealing keys, say); the error of the samples is then
/// the result.
///
/// # Errors
/// `Database`, `Places`, `Locations`, `Keys`, `Extension`, `Switches`, `Events`.
pub async fn sweep(db: &PgPool, reading: &Reading, now: DateTime<Utc>) -> Result<Swept, Error> {
    let timed = times(db, now).await?;
    given(db, reading, now).await?;
    let placed = arrivals(db, &reading.seal, now).await?;
    Ok(Swept {
        firings: timed + placed.firings,
        place_changes: placed.place_changes,
    })
}
