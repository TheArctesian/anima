//! The sample a collection produces (MET-002, MET-009, D-5, R-099).

use super::{Kind, MAX_GPU_INDEX};

/// One value of one series at one collection: the label is a mount point, an interface name
/// or a GPU index, and empty for the kinds that have none (D-5).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub kind: Kind,
    pub label: String,
    pub value: f64,
    /// The GPU of a GPU kind's sample, also its label; `None` for the other kinds (MET-009,
    /// R-099).
    pub gpu_index: Option<u8>,
}

impl Sample {
    /// A sample of `value`, or `None` when it is not a finite number of at least zero.
    /// Percent kinds are held to at most 100.
    #[must_use]
    pub fn new(kind: Kind, label: &str, value: f64) -> Option<Self> {
        Some(Self {
            kind,
            label: label.to_owned(),
            value: kept_value(kind, value)?,
            gpu_index: None,
        })
    }

    /// A sample of `value` for the GPU numbered `gpu_index` on its device, labelled with the
    /// index as decimal text, so each GPU has its own series (MET-009, R-099). `None` as for
    /// [`Sample::new`], for a kind that belongs to no GPU, and for an index above
    /// [`MAX_GPU_INDEX`] (K8 Section 4).
    #[must_use]
    pub fn gpu(kind: Kind, gpu_index: u8, value: f64) -> Option<Self> {
        if !kind.is_per_gpu() || gpu_index > MAX_GPU_INDEX {
            return None;
        }
        Some(Self {
            gpu_index: Some(gpu_index),
            ..Self::new(kind, &gpu_index.to_string(), value)?
        })
    }
}

/// The value a sample of `kind` keeps: `None` when `value` is not a finite number of at least
/// zero (`-0` included); a percent is held to at most 100 (MET-002).
pub(crate) fn kept_value(kind: Kind, value: f64) -> Option<f64> {
    if !value.is_finite() || value.is_sign_negative() {
        return None;
    }
    Some(if kind.is_percent() {
        value.min(100.0)
    } else {
        value
    })
}
