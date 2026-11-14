//! What every live read of a project repository on GitHub shares (K-E1 Section 13; PRJ-031,
//! PRJ-032): the repository's reach, the read through the engine's cache, the read stamp, the
//! K7 cursors over GitHub's pages, the fixed failure words and the cut of GitHub's text.
//!
//! Text from GitHub is other people's: it stays a plain string, is cut as K1 states, and never
//! reaches a log line or an error message (LOG-007).

mod argument;
mod checks;
pub(super) mod cursor;
mod failure;
mod filter;
pub(super) mod reach;
pub(super) mod text;

use std::sync::Arc;

use async_graphql::{Context, ID, Result, SimpleObject};
use chrono::{DateTime, Utc};
use source_exchange::{Body, Problem, Request, Response};

pub(super) use self::{
    argument::checked,
    checks::{CommitChecksView, commit_checks},
    cursor::{Cut, GITHUB_PAGE, Place, cut},
    failure::{missing, unusable},
    filter::OpenOrClosed,
    reach::{RepositoryAccess, Target, access},
};
use super::{Error, on, store::reads};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, pool, schema_dependency},
    clock::Clock,
    engine::sources::{self, Keep, Reach, Reply, Scheduling},
    fleet::events::Bus,
    ids::RepositoryId,
    paging::PageEnd,
};

/// The largest number GitHub gives a run, comment or workflow that the hub accepts (K7, IDs).
const MOST_DIGITS: u64 = 1 << 53;

/// When the hub read an answer from GitHub (PRJ-032).
#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ForgeRead")]
pub(super) struct ForgeRead {
    /// When GitHub gave this answer.
    at: DateTime<Utc>,
    /// True: the newest read failed and this is the last good answer, at most one hour old.
    stale: bool,
    /// Set when stale. Fixed words; never a provider reply (LOG-007).
    problem: Option<String>,
}

/// What a lazy field needs to read GitHub again: the repository and the root's `fresh`.
#[derive(Clone, Debug)]
pub(super) struct Asked {
    pub target: Target,
    pub fresh: bool,
}

/// The read stamp of `reply`.
pub(super) fn stamp(reply: &Reply) -> ForgeRead {
    ForgeRead {
        at: reply.at(),
        stale: reply.stale(),
        problem: reply.problem().map(failure::words),
    }
}

/// The repository `repository`, once Projects is on and it can be read now.
///
/// # Errors
/// `CONFLICT` while Projects is off or the access is not READY (with fixed words for each);
/// `VALIDATION` for an ID that is not one; `NOT_FOUND` for an unknown repository.
pub(super) async fn target(ctx: &Context<'_>, repository: &ID) -> Result<Target> {
    let db = on(ctx).await?;
    let id: RepositoryId = repository.parse().map_err(|_| {
        invalid(&[FieldError::new(
            "repository",
            "That repository id is not valid.",
        )])
    })?;
    let row = reads::reach(db, id)
        .await
        .map_err(|e| Error::from(e).into_graphql())?
        .ok_or_else(|| Error::NoRepository.into_graphql())?;
    let standing = match row.account_id {
        Some(account) => {
            let mut conn = db
                .acquire()
                .await
                .map_err(|e| Error::from(e).into_graphql())?;
            let standings = sources::standing(&mut conn, &[account])
                .await
                .map_err(internal)?;
            standings.first().map(|(_, standing)| *standing)
        }
        None => None,
    };
    match (access(row.left_out, row.gone, standing), row.account_id) {
        (RepositoryAccess::Ready, Some(account)) => Ok(Target {
            repository: id,
            account,
            name: row.full_name,
            home: row.html_url,
            default_branch: row.default_branch,
        }),
        (refused, _) => Err(coded(reach::refusal(refused), ErrorCode::Conflict)),
    }
}

/// The engine's parts for a live read, from the schema data. The vault passes through by its
/// type: this domain never opens it.
fn scheduling(ctx: &Context<'_>) -> Result<Scheduling> {
    let hub = (
        pool(ctx)?.clone(),
        schema_dependency::<Bus>(ctx)?.clone(),
        schema_dependency::<Arc<dyn Clock>>(ctx)?.clone(),
    );
    Ok(Scheduling::new(
        hub,
        Clone::clone(schema_dependency(ctx)?),
        schema_dependency::<Reach>(ctx)?.clone(),
    ))
}

/// Reads `request` for `target` through the engine's cache, as `keep` says: the reply for a
/// 200, `None` for a 404.
///
/// # Errors
/// `CONFLICT` when the provider refused the repository's name; the K7 code and fixed words of
/// [`failure::failed`] and [`failure::answered`] otherwise.
pub(super) async fn read(
    ctx: &Context<'_>,
    target: &Target,
    request: Result<Request, Problem>,
    keep: Keep,
) -> Result<Option<Reply>> {
    let request = request.map_err(|_| failure::unnamed())?;
    let reply = sources::ask(&scheduling(ctx)?, target.account, &request, keep)
        .await
        .map_err(failure::failed)?;
    Ok(failure::answered(reply.response().status)?.then_some(reply))
}

/// The body of a 200 `reply` as the provider's `parse` reads it.
///
/// # Errors
/// `UNAVAILABLE` with fixed words when the body was larger than the hub reads, or `parse`
/// cannot read it; never the provider's own words.
pub(super) fn parsed<T>(
    reply: &Reply,
    parse: fn(&Response<'_>) -> Result<T, Problem>,
) -> Result<T> {
    let response = reply.response();
    if matches!(response.body, Body::TooLarge) {
        return Err(failure::too_large());
    }
    parse(&response).map_err(|_| failure::unusable())
}

/// A GitHub number given as K1 `ID` digits (a run, comment or workflow), 1 to 2^53.
///
/// # Errors
/// A `VALIDATION` error naming `field` otherwise.
pub(super) fn digits(id: &ID, field: &'static str) -> Result<u64> {
    id.as_str()
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| id.as_str().parse::<u64>().ok())
        .flatten()
        .filter(|n| (1..=MOST_DIGITS).contains(n))
        .ok_or_else(|| invalid(&[FieldError::new(field, "Give GitHub's number, as digits.")]))
}

/// A pull request or issue number, 1 or more.
///
/// # Errors
/// A `VALIDATION` error for `number` otherwise.
pub(super) fn number(number: i32) -> Result<u64> {
    u64::try_from(number)
        .ok()
        .filter(|n| *n >= 1)
        .ok_or_else(|| invalid(&[FieldError::new("number", "Give a number of 1 or more.")]))
}

/// A K1 `Int` from GitHub's count, at most `i32::MAX`.
pub(super) fn int(count: u64) -> i32 {
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// The page size and the start of a list from `first` and `after`; `first` is 1 to `most`.
///
/// # Errors
/// One `VALIDATION` error listing each bad argument.
pub(super) fn paging(first: i32, after: Option<&str>, most: usize) -> Result<(usize, Place)> {
    let size = cursor::page_size(first, most);
    let place = size
        .as_ref()
        .map_or(Ok(Place::START), |size| cursor::start(after, *size));
    match (size, place) {
        (Ok(size), Ok(place)) => Ok((size, place)),
        (size, place) => {
            let errors: Vec<FieldError> = size.err().into_iter().chain(place.err()).collect();
            Err(invalid(&errors))
        }
    }
}

/// The edges of `cut` from the rows of one GitHub page, each made by `edge` from its cursor,
/// and where the page ends.
pub(super) fn page<T, E>(
    rows: Vec<T>,
    cut: Cut,
    edge: impl FnMut((String, T)) -> E,
) -> (Vec<E>, PageEnd) {
    let edges = cut
        .cursors
        .into_iter()
        .zip(rows.into_iter().skip(cut.rows.start).take(cut.rows.len()))
        .map(edge)
        .collect();
    let end = PageEnd {
        has_next_page: cut.has_next_page,
        end_cursor: cut.end_cursor,
    };
    (edges, end)
}
