//! The device streams: `anima device watch` (status changes) and `anima device metrics --watch`
//! (new readings). Each prints one line for each change until interrupted.

pub mod metrics;
pub mod status;
