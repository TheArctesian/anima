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
        if !value.is_finite() || value.is_sign_negative() {
            return None;
        }
        let value = if kind.is_percent() {
            value.min(100.0)
        } else {
            value
        };
        Some(Self {
            kind,
            label: label.to_owned(),
            value,
        })
    }
}
