//! The status filter, the STATUS and UPTIME orders and their cursors (CHK-004, CHK-011,
//! API-006). Pure: the readings come in as parameters.
//!
//! Every order is a (key, name) pair. The key is 0 for NAME, the severity rank for STATUS and
//! the bits of the uptime ratio for UPTIME (ratios are never negative, so their bits sort like
//! the numbers; no ratio sorts last). The unique name breaks ties, so a cursor is exact.

use std::collections::HashMap;

use crate::{
    fleet::devices::{
        FieldError,
        graphql::filter::DeviceSort,
        model::Device,
        page::{decode_cursor, encode_cursor, split_page},
    },
    ids::DeviceId,
    status::{Status, worst},
};

const SORTED_PREFIX: &str = "s1.";
const KEY_DIGITS: usize = 16;
const ALL: [Status; 5] = [
    Status::Up,
    Status::Down,
    Status::Degraded,
    Status::Maintenance,
    Status::Unknown,
];

/// What the checks domain says about the devices being ordered.
pub(super) struct Readings {
    pub(super) statuses: HashMap<DeviceId, Status>,
    pub(super) uptimes: HashMap<DeviceId, f64>,
}

impl Readings {
    fn status(&self, device: DeviceId) -> Status {
        self.statuses
            .get(&device)
            .copied()
            .unwrap_or(Status::Unknown)
    }
}

/// Where a page starts: just after the device with this key and name.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::fleet::devices::graphql) struct Position {
    key: u64,
    pub(super) name: String,
}

/// 0 for the worst status (Down) up to 4 for the best (Up), by `worst` (CHK-004).
fn status_rank(status: Status) -> u64 {
    let worse = ALL
        .into_iter()
        .filter(|other| *other != status && worst(status, *other) == *other)
        .count();
    u64::try_from(worse).unwrap_or(u64::MAX)
}

fn key_of(sort: DeviceSort, device: &Device, readings: &Readings) -> u64 {
    match sort {
        DeviceSort::Name => 0,
        DeviceSort::Status => status_rank(readings.status(device.id)),
        DeviceSort::Uptime => readings
            .uptimes
            .get(&device.id)
            .map_or(u64::MAX, |ratio| ratio.to_bits()),
    }
}

/// The devices whose status is one of `wanted` (all when it is empty), in the order of `sort`.
pub(super) fn arrange(
    devices: Vec<Device>,
    sort: DeviceSort,
    readings: &Readings,
    wanted: &[Status],
) -> Vec<(u64, Device)> {
    let mut kept: Vec<_> = devices
        .into_iter()
        .filter(|device| wanted.is_empty() || wanted.contains(&readings.status(device.id)))
        .map(|device| (key_of(sort, &device, readings), device))
        .collect();
    kept.sort_by(|(a_key, a), (b_key, b)| (a_key, &a.name).cmp(&(b_key, &b.name)));
    kept
}

/// One page of arranged devices after `after`, each with its cursor, and whether more follow.
pub(super) fn page(
    arranged: Vec<(u64, Device)>,
    sort: DeviceSort,
    after: Option<&Position>,
    size: usize,
) -> (Vec<(String, Device)>, bool) {
    let start = after.map_or(0, |at| {
        arranged.partition_point(|(key, device)| (*key, device.name.as_str()) <= (at.key, &at.name))
    });
    let shown = arranged
        .into_iter()
        .skip(start)
        .take(size.saturating_add(1))
        .map(|(key, device)| (cursor(sort, key, &device.name), device))
        .collect();
    let page = split_page(shown, size);
    (page.rows, page.has_next_page)
}

fn tag(sort: DeviceSort) -> &'static str {
    match sort {
        DeviceSort::Name => "name",
        DeviceSort::Status => "status",
        DeviceSort::Uptime => "uptime",
    }
}

/// The opaque cursor after this device. NAME keeps the cursor of the SQL path, so the two
/// paths take each other's cursors.
pub(super) fn cursor(sort: DeviceSort, key: u64, name: &str) -> String {
    match sort {
        DeviceSort::Name => encode_cursor(name),
        _ => format!(
            "{SORTED_PREFIX}{}.{key:0KEY_DIGITS$x}.{}",
            tag(sort),
            encode_cursor(name)
        ),
    }
}

/// The position a cursor points after, if the cursor was made for this sort.
///
/// # Errors
/// An `after` error when the text is not a cursor of this sort.
pub(in crate::fleet::devices::graphql) fn decode(
    sort: DeviceSort,
    text: &str,
) -> Result<Position, FieldError> {
    if sort == DeviceSort::Name {
        return Ok(Position {
            key: 0,
            name: decode_cursor(text)?,
        });
    }
    let bad = || {
        FieldError::new(
            "after",
            "That page cursor is not valid for this order. Start again from the first page.",
        )
    };
    let rest = text.strip_prefix(SORTED_PREFIX).ok_or_else(bad)?;
    let (made_for, rest) = rest.split_once('.').ok_or_else(bad)?;
    let (key, name) = rest.split_once('.').ok_or_else(bad)?;
    if made_for != tag(sort)
        || key.len() != KEY_DIGITS
        || !key.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(bad());
    }
    Ok(Position {
        key: u64::from_str_radix(key, 16).map_err(|_| bad())?,
        name: decode_cursor(name).map_err(|_| bad())?,
    })
}
