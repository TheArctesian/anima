//! The sweep of each minute (K-E6 Sections 3 and 4; REM-002, REM-006): the time rules, and the
//! place rules from the location samples that arrived since the last sweep. The caller gives
//! `now`. A hub that was stopped fires a time rule that it missed one time, and a late import
//! of old samples makes no arrival in the past.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::{
    Error,
    model::{PlaceChange, Trigger},
    place_state::place_changes,
    sending::{WasKnown, fire},
    store,
};
use crate::engine::{locations, places};

/// The cursor of the location samples in `reminders.cursor`.
const SAMPLES: &str = "locations";
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

/// One sweep at `now`: the time rules, then the place rules. The time rules are stored also
/// when the samples cannot be read (no sealing keys, say); the error of the samples is then
/// the result.
///
/// # Errors
/// `Database`, `Places`, `Locations`.
pub async fn sweep(
    db: &PgPool,
    seal: &locations::Seal,
    now: DateTime<Utc>,
) -> Result<Swept, Error> {
    let timed = times(db, now).await?;
    let placed = arrivals(db, seal, now).await?;
    Ok(Swept {
        firings: timed + placed.firings,
        place_changes: placed.place_changes,
    })
}
