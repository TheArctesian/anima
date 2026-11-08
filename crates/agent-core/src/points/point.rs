//! One point: what the samples of each series in one 10-second window come to (MET-014,
//! MET-015, K8 Section 4).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::SeriesKey;

/// The 10 seconds that end at [`Point::at`], for each series that had a sample in them. A series
/// of the device that is not here had none, and goes in a batch as `null` (K8 Section 4).
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    at: DateTime<Utc>,
    values: BTreeMap<SeriesKey, Summary>,
}

/// The samples of one series in one window (MET-014).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Summary {
    /// Their mean: finite, at least 0, at most 100 for a percent kind.
    pub mean: f64,
    /// Their highest value, kept only for the kinds with [`Kind::has_max`](super::Kind::has_max);
    /// never below `mean`.
    pub max: Option<f64>,
}

impl Point {
    pub(crate) const fn new(at: DateTime<Utc>, values: BTreeMap<SeriesKey, Summary>) -> Self {
        Self { at, values }
    }

    /// The end of the window, a multiple of 10 seconds of UTC.
    #[must_use]
    pub const fn at(&self) -> DateTime<Utc> {
        self.at
    }

    /// Each series that had a sample in the window, with what its samples came to.
    #[must_use]
    pub const fn values(&self) -> &BTreeMap<SeriesKey, Summary> {
        &self.values
    }
}
