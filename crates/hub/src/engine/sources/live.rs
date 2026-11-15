//! Live reads and named changes of a source, for the Work, Code and History tabs of a project
//! (K-E1 Section 13; PRJ-027 to PRJ-035, OD-E14).
//!
//! A live read is a `GET` through the transport with the account's installation token, served
//! from a short cache in hub memory while it is fresh. A change is a checked [`Change`] the
//! provider renders and the transport sends; the caller names nothing else. Neither touches a
//! sync: the live path never claims an account, never writes `sources.accounts` or
//! `sources.sync_runs`, and changes no schedule, failure count or health. A 401 drops the
//! in-memory token, as a sync's does, and writes nothing.

mod budget;
mod model;
mod pace;
mod read;
mod replies;
#[cfg(test)]
mod tests;
pub(super) mod token;
mod write;

use std::sync::{Arc, Mutex, PoisonError};

use sqlx::PgConnection;

use self::{budget::Budget, pace::Pace, replies::Replies};
pub use self::{
    model::{Keep, LiveFailure, Reply, Reserve, Standing},
    read::ask,
    write::change,
};
use super::{
    Scheduling, Transport,
    model::{Account, ErrorKind},
    store,
};
use crate::ids::SourceAccountId;

/// The live path's state, held by the transport beside its tokens: the cache, the budget and
/// the pace of every account. Cheap to clone; clones share it.
#[derive(Clone, Default)]
pub(in crate::engine::sources) struct Live {
    replies: Arc<Mutex<Replies>>,
    budget: Arc<Mutex<Budget>>,
    pace: Pace,
}

impl Live {
    fn replies<T>(&self, f: impl FnOnce(&mut Replies) -> T) -> T {
        f(&mut self.replies.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn budget<T>(&self, f: impl FnOnce(&mut Budget) -> T) -> T {
        f(&mut self.budget.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Drops the held answers of `account` whose path is `path_prefix` or lies below it, after a
/// change made them old (K-E1 Section 13.6). `/repos/o/r/pulls` drops `/repos/o/r/pulls/12`
/// but not `/repos/o/r/pulls2`.
pub fn forget(transport: &Transport, account: SourceAccountId, path_prefix: &str) {
    transport
        .live()
        .replies(|replies| replies.forget(account, path_prefix));
}

/// Drops every held answer of `account`, once it is removed.
pub(in crate::engine::sources) fn forget_account(transport: &Transport, account: SourceAccountId) {
    transport
        .live()
        .replies(|replies| replies.forget_account(account));
}

/// Whether each of `accounts` can be read now, from the stored rows; nothing is written.
///
/// # Errors
/// [`LiveFailure::Store`] when the rows cannot be read.
pub async fn standing(
    conn: &mut PgConnection,
    accounts: &[SourceAccountId],
) -> Result<Vec<(SourceAccountId, Standing)>, LiveFailure> {
    let mut standings = Vec::with_capacity(accounts.len());
    for &id in accounts {
        let account = store::account(&mut *conn, id)
            .await
            .map_err(|_| LiveFailure::Store)?;
        standings.push((id, standing_of(account.as_ref())));
    }
    Ok(standings)
}

/// The standing of a stored account, in the order the health uses: paused first, then a
/// refused key.
fn standing_of(account: Option<&Account>) -> Standing {
    match account {
        None => Standing::NotFound,
        Some(account) if account.paused_at.is_some() => Standing::Paused,
        Some(account)
            if account.credential_id.is_none() || account.error_kind() == Some(ErrorKind::Auth) =>
        {
            Standing::NeedsReconnect
        }
        Some(_) => Standing::Ready,
    }
}

/// The stored account and its source's declaration, when the account may be read now.
async fn reachable(
    s: &Scheduling,
    id: SourceAccountId,
) -> Result<(Account, source_exchange::Declaration), LiveFailure> {
    let account = store::account(&s.pool, id)
        .await
        .map_err(|_| LiveFailure::Store)?
        .ok_or(LiveFailure::NotFound)?;
    if account.paused_at.is_some() {
        return Err(LiveFailure::Paused);
    }
    let declaration = *s
        .registry
        .declaration(&account.source)
        .ok_or(LiveFailure::Unusable)?;
    Ok((account, declaration))
}

/// What a failure of the token function means for a live request.
const fn failed(failure: super::run::Failure, now: chrono::DateTime<chrono::Utc>) -> LiveFailure {
    match failure.kind {
        ErrorKind::Auth => LiveFailure::KeyRefused,
        ErrorKind::RateLimit => LiveFailure::Limited {
            until: match failure.until {
                Some(until) => until,
                None => now,
            },
        },
        ErrorKind::Network => LiveFailure::Unreachable,
        ErrorKind::Provider => LiveFailure::Unusable,
        ErrorKind::Store => LiveFailure::Store,
    }
}
