//! From 2-second samples to one point per 10-second window of UTC (MET-014, K8 Section 7).
//!
//! Pure: the time of each sampling is a parameter (CQ-021).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::{MAX_SERIES, Point, SeriesKey, Summary, sample::kept_value};

/// The length of a window, in seconds (MET-014).
const WINDOW_SECONDS: i64 = 10;

/// The open window: the samples taken since the last point, per series.
#[derive(Debug, Default)]
pub struct Window {
    end: Option<DateTime<Utc>>,
    columns: BTreeMap<SeriesKey, Column>,
}

/// The running figures of one series in the open window.
#[derive(Clone, Copy, Debug)]
struct Column {
    count: u32,
    mean: f64,
    lowest: f64,
    highest: f64,
}

impl Window {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the `values` of one sampling taken at `at`. A sample belongs to the window that
    /// ends at or after it, on a multiple of 10 seconds. When `at` falls in another window than
    /// the open one (time moved on, or the clock went back), the open window closes first and
    /// its point is returned; a window in which no value was kept makes no point.
    ///
    /// A value that is not finite or is below 0 is left out, and a percent is held to 100, as
    /// [`Sample::new`](super::Sample::new) does. A window keeps at most [`MAX_SERIES`] series; later new ones are
    /// left out.
    pub fn add(
        &mut self,
        at: DateTime<Utc>,
        values: impl IntoIterator<Item = (SeriesKey, f64)>,
    ) -> Option<Point> {
        let end = window_end(at);
        let closed = match self.end {
            Some(open) if open != end => self.close(open),
            _ => None,
        };
        self.end = Some(end);
        for (key, value) in values {
            self.take(key, value);
        }
        closed
    }

    fn take(&mut self, key: SeriesKey, value: f64) {
        let Some(value) = kept_value(key.kind(), value) else {
            return;
        };
        if let Some(column) = self.columns.get_mut(&key) {
            column.count = column.count.saturating_add(1);
            // A running mean: never overflows for values of at least 0.
            column.mean += (value - column.mean) / f64::from(column.count);
            column.lowest = column.lowest.min(value);
            column.highest = column.highest.max(value);
        } else if self.columns.len() < MAX_SERIES {
            let column = Column {
                count: 1,
                mean: value,
                lowest: value,
                highest: value,
            };
            self.columns.insert(key, column);
        }
    }

    fn close(&mut self, end: DateTime<Utc>) -> Option<Point> {
        let columns = std::mem::take(&mut self.columns);
        if columns.is_empty() {
            return None;
        }
        let values = columns
            .into_iter()
            .map(|(key, column)| {
                let summary = Summary {
                    // Rounding cannot put the mean outside the samples it comes from.
                    mean: column.mean.clamp(column.lowest, column.highest),
                    max: key.kind().has_max().then_some(column.highest),
                };
                (key, summary)
            })
            .collect();
        Some(Point::new(end, values))
    }
}

/// The end of the window `at` belongs to: `at` itself when it is a whole multiple of 10
/// seconds, else the next such time.
fn window_end(at: DateTime<Utc>) -> DateTime<Utc> {
    if is_window_end(at) {
        return at;
    }
    let end = (at.timestamp().div_euclid(WINDOW_SECONDS) + 1) * WINDOW_SECONDS;
    DateTime::from_timestamp(end, 0).unwrap_or(at)
}

/// Whether `at` is a whole multiple of 10 seconds of UTC, as every point's `at` is (K8 Section 4).
pub(crate) fn is_window_end(at: DateTime<Utc>) -> bool {
    at.timestamp().rem_euclid(WINDOW_SECONDS) == 0 && at.timestamp_subsec_nanos() == 0
}
