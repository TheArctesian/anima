//! `Query.deliveries`: the delivery log, newest first, a page at a time, of every channel or
//! of one (NOT-004, API-006, R-014).
//!
//! A cursor is `d1.` + the delivery's time in microseconds since the epoch + `.` + its id as 32
//! lower-case hex digits. It is parsed strictly: only the exact text `encode_cursor` writes is
//! read; any other text is a `VALIDATION` error on `after`, never a panic or a guess.

use async_graphql::{Context, ID, Result, SimpleObject};
use chrono::DateTime;
use uuid::Uuid;

use super::{delivery::DeliveryNode, to_graphql};
use crate::{
    api_errors::{FieldError, invalid, pool},
    fleet::notifications::{Delivery, DeliveryCursor, store},
    ids::ChannelId,
    paging::PageEnd,
};

/// K1's default for `first`.
pub(super) const DEFAULT_FIRST: i32 = 50;
/// The most deliveries one page holds (K1 bounds, as every list).
const MAX_FIRST: usize = 200;
const PREFIX: &str = "d1.";
const HEX_DIGITS: usize = 32;

const FIRST_MESSAGE: &str = "Ask for between 1 and 200 entries per page.";
const AFTER_MESSAGE: &str = "That page cursor is not valid. Start again from the first page.";
const CHANNEL_MESSAGE: &str = "That channel id is not valid.";

#[derive(SimpleObject)]
#[graphql(name = "DeliveryEdge")]
struct DeliveryEdge {
    cursor: String,
    node: DeliveryNode,
}

/// K1 `DeliveryConnection`.
#[derive(SimpleObject)]
#[graphql(name = "DeliveryConnection")]
pub(super) struct DeliveryConnection {
    edges: Vec<DeliveryEdge>,
    page_info: PageEnd,
}

impl DeliveryConnection {
    /// The page of `size` from `fetched`, which holds up to one row more to tell whether
    /// another page follows.
    fn of(mut fetched: Vec<Delivery>, size: usize) -> Self {
        let has_next_page = fetched.len() > size;
        fetched.truncate(size);
        let edges: Vec<_> = fetched
            .into_iter()
            .map(|delivery| DeliveryEdge {
                cursor: encode_cursor(DeliveryCursor {
                    at: delivery.at,
                    id: delivery.id,
                }),
                node: delivery.into(),
            })
            .collect();
        let end_cursor = edges.last().map(|edge| edge.cursor.clone());
        Self {
            edges,
            page_info: PageEnd {
                has_next_page,
                end_cursor,
            },
        }
    }
}

/// The checked arguments of `Query.deliveries`.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct DeliveriesArgs {
    pub(super) channel: Option<ChannelId>,
    pub(super) size: usize,
    pub(super) after: Option<DeliveryCursor>,
}

/// Reads the arguments, reporting every bad one at once.
pub(super) fn deliveries_args(
    channel_id: Option<&ID>,
    first: Option<i32>,
    after: Option<&str>,
) -> Result<DeliveriesArgs, Vec<FieldError>> {
    let channel = channel_id
        .map(|id| {
            id.parse()
                .map_err(|_| FieldError::new("channelId", CHANNEL_MESSAGE))
        })
        .transpose();
    let size = usize::try_from(first.unwrap_or(DEFAULT_FIRST))
        .ok()
        .filter(|size| (1..=MAX_FIRST).contains(size))
        .ok_or_else(|| FieldError::new("first", FIRST_MESSAGE));
    let after = after
        .map(|text| decode_cursor(text).ok_or_else(|| FieldError::new("after", AFTER_MESSAGE)))
        .transpose();
    match (channel, size, after) {
        (Ok(channel), Ok(size), Ok(after)) => Ok(DeliveriesArgs {
            channel,
            size,
            after,
        }),
        (channel, size, after) => Err(channel
            .err()
            .into_iter()
            .chain(size.err())
            .chain(after.err())
            .collect()),
    }
}

/// The page `args` asks for.
pub(super) async fn page(ctx: &Context<'_>, args: DeliveriesArgs) -> Result<DeliveryConnection> {
    let limit = i64::try_from(args.size.saturating_add(1)).unwrap_or(i64::MAX);
    let fetched = store::deliveries_page(pool(ctx)?, args.channel, args.after, limit)
        .await
        .map_err(to_graphql)?;
    Ok(DeliveryConnection::of(fetched, args.size))
}

/// Reads the arguments or answers `VALIDATION`, then the page.
pub(super) async fn resolve(
    ctx: &Context<'_>,
    channel_id: Option<&ID>,
    first: Option<i32>,
    after: Option<&str>,
) -> Result<DeliveryConnection> {
    let args = deliveries_args(channel_id, first, after).map_err(|errors| invalid(&errors))?;
    page(ctx, args).await
}

/// The opaque cursor just after `cursor`.
pub(super) fn encode_cursor(cursor: DeliveryCursor) -> String {
    format!(
        "{PREFIX}{}.{}",
        cursor.at.timestamp_micros(),
        Uuid::from(cursor.id).simple()
    )
}

/// The position `text` names, when it is a cursor [`encode_cursor`] could have written.
pub(super) fn decode_cursor(text: &str) -> Option<DeliveryCursor> {
    let (micros, hex) = text.strip_prefix(PREFIX)?.split_once('.')?;
    let digits = micros.strip_prefix('-').unwrap_or(micros);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if hex.len() != HEX_DIGITS || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let at = DateTime::from_timestamp_micros(micros.parse().ok()?)?;
    let id = Uuid::try_parse(hex).ok()?.into();
    let cursor = DeliveryCursor { at, id };
    // One position, one cursor: leading zeros, `-0` and upper-case digits are refused.
    (encode_cursor(cursor) == text).then_some(cursor)
}
