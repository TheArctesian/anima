//! Resource metrics of a device: CPU, memory, disk, network, uptime and load, read from the
//! output of fixed read-only commands run over SSH (spec 6.6, HUB-004, MET-001, MET-002).
//!
//! This part is pure: [`parse`] turns command output into a [`Reading`], and [`samples`] turns
//! two readings and the time between them into [`Sample`]s. The device is not trusted, so no
//! reader panics and none fails on a bad line.

mod model;
pub mod parse;
mod rates;
mod reading;
mod series;
#[cfg(test)]
mod tests;

pub use model::{GpuVendor, MetricKind, MetricRange, Sample};
pub use rates::samples;
pub use reading::{Cpu, Disk, Interface, Load, Memory, Reading};
pub use series::MAX_LABELS;
