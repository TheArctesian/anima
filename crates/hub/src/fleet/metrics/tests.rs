//! Tests of the metrics kinds, the stored series, their aggregates and retention.

use crate::fleet::metrics::MetricKind;

mod aggregates;
mod database;
mod faces;
mod gpu_labels;
mod gpu_reads;
mod gpu_series;
mod kinds;
mod privileges;
mod retention;
mod storage;

/// Asserts two floats are the same number, apart from rounding of the last digits.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}
