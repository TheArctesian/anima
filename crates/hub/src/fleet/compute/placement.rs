//! Where a model can run: the pure fit estimate (FIT-001 to FIT-005, CMP-011). It reads no
//! store and no clock; the caller gives the inventory, the measurements and the configuration.

mod fit;
mod overhead;
#[cfg(test)]
mod tests;

pub use fit::{FitError, FitRequest, FitVerdict, KvShape, estimate_fit};
pub use overhead::{OverheadEntry, OverheadTable};
