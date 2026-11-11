//! `Query.recordings`: the recording catalogue, newest first, a page at a time, of every
//! device or of one (K1, RA-041, API-006, D-15).
//!
//! A cursor is `r1.` + the recording's start in microseconds since the epoch + `.` + its id as
//! 32 lower-case hex digits. It is parsed strictly: only the exact text `encode_cursor` writes
//! is read; any other text is a `VALIDATION` error on `after`, never a panic or a guess.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, Object, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    api_errors::FieldError,
    ids::DeviceId,
    paging::PageEnd,
    sessions::{
        Protocol,
        recording::{
            format::Format,
            model::{Catalogued, Position},
        },
    },
};

/// K1's default for `first`.
pub(super) const DEFAULT_FIRST: i32 = 50;
/// The most recordings one page holds (K1 bounds, as every list).
const MAX_FIRST: usize = 200;
const PREFIX: &str = "r1.";
const HEX_DIGITS: usize = 32;
/// The start times a cursor may hold, in microseconds: `0001-01-01T00:00:00Z` to
/// `9999-12-31T23:59:59.999999Z`. Narrower than PostgreSQL's `timestamptz`, so every cursor
/// that decodes binds as a query parameter (API-010).
const CURSOR_MICROS: std::ops::RangeInclusive<i64> =
    -62_135_596_800_000_000..=253_402_300_799_999_999;

const FIRST_MESSAGE: &str = "Ask for between 1 and 200 recordings per page.";
const AFTER_MESSAGE: &str = "That page cursor is not valid. Start again from the first page.";
const DEVICE_MESSAGE: &str = "That device id is not valid.";

/// K1 `RecordingFormat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "RecordingFormat")]
enum RecordingFormat {
    Guacamole,
    Asciicast,
}

impl From<Format> for RecordingFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Guacamole => Self::Guacamole,
            Format::Asciicast => Self::Asciicast,
        }
    }
}

/// K1 `Recording`: the row without its key.
#[Object(name = "Recording")]
impl Catalogued {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn session_id(&self) -> ID {
        ID(self.session.to_string())
    }

    async fn device_id(&self) -> Option<ID> {
        self.device.map(|device| ID(device.to_string()))
    }

    /// The device's name when the session started; kept after the device is deleted (D-15).
    async fn device_name(&self) -> &str {
        &self.device_name
    }

    async fn protocol(&self) -> Protocol {
        self.protocol
    }

    async fn format(&self) -> RecordingFormat {
        self.format.into()
    }

    async fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    async fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    /// Bytes of plaintext; Float above 2^31.
    #[expect(
        clippy::cast_precision_loss,
        reason = "K1 sends the size as Float; exact to 2^53 bytes, far past any recording"
    )]
    async fn size_bytes(&self) -> f64 {
        self.size_bytes as f64
    }

    /// False when the hub stopped before the last part (K4 §11).
    async fn complete(&self) -> bool {
        self.complete
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

#[derive(SimpleObject)]
#[graphql(name = "RecordingEdge")]
struct RecordingEdge {
    cursor: String,
    node: Catalogued,
}

/// K1 `RecordingConnection`.
#[derive(SimpleObject)]
#[graphql(name = "RecordingConnection")]
pub(super) struct RecordingConnection {
    edges: Vec<RecordingEdge>,
    page_info: PageEnd,
}

impl RecordingConnection {
    /// The page of `size` from `fetched`, which holds up to one row more to tell whether
    /// another page follows.
    pub(super) fn of(mut fetched: Vec<Catalogued>, size: usize) -> Self {
        let has_next_page = fetched.len() > size;
        fetched.truncate(size);
        let edges: Vec<_> = fetched
            .into_iter()
            .map(|node| RecordingEdge {
                cursor: encode_cursor(Position::of(&node)),
                node,
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

/// The checked arguments of `Query.recordings`.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RecordingsArgs {
    pub(super) device: Option<DeviceId>,
    pub(super) size: usize,
    pub(super) after: Option<Position>,
}

impl RecordingsArgs {
    /// The rows to fetch: one more than the page, to tell whether another follows.
    pub(super) fn limit(&self) -> i64 {
        i64::try_from(self.size.saturating_add(1)).unwrap_or(i64::MAX)
    }
}

/// Reads the arguments, reporting every bad one at once.
pub(super) fn recordings_args(
    device_id: Option<&ID>,
    first: Option<i32>,
    after: Option<&str>,
) -> Result<RecordingsArgs, Vec<FieldError>> {
    let device = device_id
        .map(|id| {
            id.parse()
                .map_err(|_| FieldError::new("deviceId", DEVICE_MESSAGE))
        })
        .transpose();
    let size = usize::try_from(first.unwrap_or(DEFAULT_FIRST))
        .ok()
        .filter(|size| (1..=MAX_FIRST).contains(size))
        .ok_or_else(|| FieldError::new("first", FIRST_MESSAGE));
    let after = after
        .map(|text| decode_cursor(text).ok_or_else(|| FieldError::new("after", AFTER_MESSAGE)))
        .transpose();
    match (device, size, after) {
        (Ok(device), Ok(size), Ok(after)) => Ok(RecordingsArgs {
            device,
            size,
            after,
        }),
        (device, size, after) => Err(device
            .err()
            .into_iter()
            .chain(size.err())
            .chain(after.err())
            .collect()),
    }
}

/// The opaque cursor just after `position`.
pub(in crate::sessions::recording) fn encode_cursor(position: Position) -> String {
    format!(
        "{PREFIX}{}.{}",
        position.started_at.timestamp_micros(),
        Uuid::from(position.id).simple()
    )
}

/// The position `text` names, when it is a cursor [`encode_cursor`] could have written.
pub(in crate::sessions::recording) fn decode_cursor(text: &str) -> Option<Position> {
    let (micros, hex) = text.strip_prefix(PREFIX)?.split_once('.')?;
    let digits = micros.strip_prefix('-').unwrap_or(micros);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if hex.len() != HEX_DIGITS || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let micros: i64 = micros.parse().ok()?;
    if !CURSOR_MICROS.contains(&micros) {
        return None;
    }
    let started_at = DateTime::from_timestamp_micros(micros)?;
    let id = Uuid::try_parse(hex).ok()?.into();
    let position = Position { started_at, id };
    // One position, one cursor: leading zeros, `-0` and upper-case digits are refused.
    (encode_cursor(position) == text).then_some(position)
}
