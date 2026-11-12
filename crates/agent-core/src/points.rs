//! From readings to pushed points (MET-002, MET-014, MET-015, K8 Sections 4 and 7).
//!
//! [`samples`] turns two readings and the time between them into [`Sample`]s, by the one rule
//! the hub's SSH collector and the agent share (R-092 D-3). A [`Window`] makes one [`Point`] of
//! each 10 seconds of samples, and the [`Backlog`] keeps the points until the hub accepts them.
//! Nothing here reads the clock, sleeps or does I/O (CQ-021).

mod activity;
mod backlog;
mod key;
mod kind;
mod labels;
mod point;
mod rates;
mod sample;
#[cfg(test)]
mod tests;
mod window;

pub use backlog::{BATCH_POINTS, Backlog, MAX_AGE, MAX_POINTS, Outgoing, Receipt};
pub(crate) use key::is_label;
pub use key::{MAX_GPU_INDEX, MAX_SERIES, SeriesKey};
pub use kind::Kind;
pub use labels::MAX_LABELS;
pub(crate) use labels::is_operator_mount;
pub use point::{Point, Summary};
pub use rates::samples;
pub use sample::{Sample, ceiling};
pub use window::Window;
pub(crate) use window::is_window_end;
