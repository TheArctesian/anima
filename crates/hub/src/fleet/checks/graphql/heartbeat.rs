//! `Heartbeat` and the connection `Check.heartbeats` returns, paged by a keyset cursor on the
//! time of the beat, newest first (API-006).

use async_graphql::{Object, SimpleObject};
use chrono::{DateTime, Datelike as _, Utc};

use crate::{
    fleet::checks::{FieldError, Heartbeat},
    paging::PageEnd,
    status::Status,
};

const MAX_FIRST: usize = 500;
const CURSOR_PREFIX: &str = "h1.";
/// The years a stored beat can have; a cursor outside them is not one we made, and the
/// database would refuse it (API-010).
const EARLIEST_YEAR: i32 = 1970;
const LATEST_YEAR: i32 = 9999;

#[Object]
impl Heartbeat {
    async fn at(&self) -> DateTime<Utc> {
        self.at
    }

    async fn status(&self) -> Status {
        self.status
    }

    async fn response_time_ms(&self) -> Option<i32> {
        self.response_time_ms.and_then(|ms| i32::try_from(ms).ok())
    }

    async fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[derive(SimpleObject)]
#[graphql(name = "HeartbeatEdge")]
pub(super) struct HeartbeatEdge {
    cursor: String,
    node: Heartbeat,
}

#[derive(SimpleObject)]
#[graphql(name = "HeartbeatConnection")]
pub(super) struct HeartbeatConnection {
    edges: Vec<HeartbeatEdge>,
    page_info: PageEnd,
}

fn encode_cursor(at: DateTime<Utc>) -> String {
    format!("{CURSOR_PREFIX}{}", at.timestamp_micros())
}

fn decode_cursor(cursor: &str) -> Option<DateTime<Utc>> {
    let micros: i64 = cursor.strip_prefix(CURSOR_PREFIX)?.parse().ok()?;
    DateTime::from_timestamp_micros(micros)
        .filter(|at| (EARLIEST_YEAR..=LATEST_YEAR).contains(&at.year()))
}

/// The page size (1 to 500) and the time the page starts after, or every argument in error.
pub(super) fn paging(
    first: i32,
    after: Option<&str>,
) -> Result<(usize, Option<DateTime<Utc>>), Vec<FieldError>> {
    let size = usize::try_from(first)
        .ok()
        .filter(|size| (1..=MAX_FIRST).contains(size))
        .ok_or_else(|| {
            FieldError::new(
                "first",
                format!("Ask for between 1 and {MAX_FIRST} items per page."),
            )
        });
    let after = after
        .map(|cursor| {
            decode_cursor(cursor).ok_or_else(|| {
                FieldError::new(
                    "after",
                    "That page cursor is not valid. Start again from the first page.",
                )
            })
        })
        .transpose();
    match (size, after) {
        (Ok(size), Ok(after)) => Ok((size, after)),
        (size, after) => Err(size.err().into_iter().chain(after.err()).collect()),
    }
}

/// A page from rows fetched with limit `size + 1`.
pub(super) fn connection(mut fetched: Vec<Heartbeat>, size: usize) -> HeartbeatConnection {
    let has_next_page = fetched.len() > size;
    fetched.truncate(size);
    let edges: Vec<_> = fetched
        .into_iter()
        .map(|node| HeartbeatEdge {
            cursor: encode_cursor(node.at),
            node,
        })
        .collect();
    let end_cursor = edges.last().map(|edge| edge.cursor.clone());
    HeartbeatConnection {
        edges,
        page_info: PageEnd {
            has_next_page,
            end_cursor,
        },
    }
}
