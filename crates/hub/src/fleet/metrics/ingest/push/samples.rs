//! The series and samples an accepted batch stores (MET-015, K8 Section 4, R-099). Pure.
//!
//! Each value becomes a [`Sample`] by the SSH collector's rule for its kind, label and GPU: a
//! disk, network or drive series keeps its label, a GPU series is labelled with its GPU index as
//! decimal text (the batch cannot choose that label: the check refused it), and every other
//! kind has the empty label.

use std::collections::BTreeMap;

use agent_core::points::{Point, SeriesKey};

use crate::fleet::metrics::{Sample, store::PointValue};

/// What a batch stores: one sample of each series it has a value of, so the series rows can be
/// made, and every value at its time.
pub(super) struct Rows {
    pub(super) series: Vec<Sample>,
    pub(super) values: Vec<PointValue>,
}

/// The rows of `points`, the accepted points of a batch. A value no sample keeps is left out,
/// as a collection leaves it out.
pub(super) fn rows(points: &[Point]) -> Rows {
    let mut series: BTreeMap<&SeriesKey, Sample> = BTreeMap::new();
    let mut values = Vec::new();
    for point in points {
        for (key, summary) in point.values() {
            let Some(sample) = sample_of(key, summary.mean) else {
                continue;
            };
            series.entry(key).or_insert_with(|| sample.clone());
            values.push(PointValue {
                at: point.at(),
                sample,
                max: summary.max,
            });
        }
    }
    Rows {
        series: series.into_values().collect(),
        values,
    }
}

/// The sample of `mean` in the series `key`, labelled as the SSH collector labels it.
fn sample_of(key: &SeriesKey, mean: f64) -> Option<Sample> {
    match key.gpu() {
        Some(index) => Sample::gpu(key.kind(), index, mean),
        None => Sample::new(key.kind(), key.label().unwrap_or_default(), mean),
    }
}
