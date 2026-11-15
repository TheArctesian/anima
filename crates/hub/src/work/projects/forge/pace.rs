//! How a change spends GitHub's requests, by who asked (K-E1 Section 13.1): the operator's own
//! change (a login or browser token) may spend the sync's reserve; a change made with an API
//! key keeps it, and one key makes at most [`CHANGES_A_MINUTE`] changes a minute, so a script
//! or an agent in a loop cannot take the account's budget from the sync and the operator.
//!
//! The count is in hub memory, in fixed windows of a minute from a key's first change; the
//! time is a parameter (CQ-021) and no lock is held across an await.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use async_graphql::{Context, Result};
use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    api_errors::{ErrorCode, coded, schema_dependency},
    clock::Clock,
    engine::sources::Reserve,
    fleet::access::Identity,
    ids::TokenId,
};

/// The most changes one API key makes in a minute: a person at a keyboard makes fewer, and
/// with the account's one second between changes it leaves two thirds of the pace to others.
pub const CHANGES_A_MINUTE: u32 = 20;
const MINUTE: TimeDelta = TimeDelta::minutes(1);

const PACED: &str = "This key makes at most 20 changes a minute. Try again in a minute.";

/// One key's minute: when it began and the changes made in it.
#[derive(Clone, Copy, Debug)]
struct Window {
    began: DateTime<Utc>,
    made: u32,
}

/// The changes each API key made in its current minute. Schema data: one for the hub.
#[derive(Default)]
pub struct KeyChanges(Mutex<HashMap<TokenId, Window>>);

impl KeyChanges {
    /// Counts one change of `key` at `now`; `false` when the key made its
    /// [`CHANGES_A_MINUTE`] in the minute that holds `now`. Keys whose minute ended are let go,
    /// so the map holds only the keys that changed something in the last minute.
    #[must_use]
    pub fn take(&self, key: TokenId, now: DateTime<Utc>) -> bool {
        let mut windows = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        windows.retain(|_, window| now < window.began + MINUTE);
        let window = windows.entry(key).or_insert(Window {
            began: now,
            made: 0,
        });
        if window.made >= CHANGES_A_MINUTE {
            return false;
        }
        window.made += 1;
        true
    }
}

/// How the change of this request spends the account's requests: the operator's may spend the
/// reserve; an API key's keeps it, once the key's pace allows one more change.
///
/// # Errors
/// `UNAVAILABLE` with fixed words when the key made its changes for this minute; nothing is
/// sent or recorded.
pub(super) fn reserve(ctx: &Context<'_>) -> Result<Reserve> {
    let Some(key) = schema_dependency::<Identity>(ctx)?.api_key() else {
        return Ok(Reserve::Spendable);
    };
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    if schema_dependency::<KeyChanges>(ctx)?.take(key, now) {
        Ok(Reserve::Kept)
    } else {
        Err(coded(PACED, ErrorCode::Unavailable))
    }
}
