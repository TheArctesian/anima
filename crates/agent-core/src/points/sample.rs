//! The sample a collection produces (MET-002, D-5).

use super::Kind;

/// One value of one series at one collection: the label is a mount point or an interface name,
/// and empty for the kinds that have none (D-5).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub kind: Kind,
    pub label: String,
    pub value: f64,
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
