//! From readings to samples (MET-002).
//!
//! [`samples`] turns two readings and the time between them into [`Sample`]s, by the one rule
//! the hub's SSH collector and the agent share (R-092 D-3).

mod kind;
mod labels;
mod rates;
mod sample;
#[cfg(test)]
mod tests;

pub use kind::Kind;
pub use labels::MAX_LABELS;
pub use rates::samples;
pub use sample::Sample;
