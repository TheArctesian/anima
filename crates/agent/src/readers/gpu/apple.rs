//! The place of the Apple GPU reader (MET-006, MET-016, R-092 D-4).
//!
//! WP-54 T5 writes it: one run of `ioreg -r -d 1 -c IOAccelerator` every 2 seconds, read by
//! `agent_core::readings::gpu::apple::gpus`, with no `unsafe`. Until then macOS reports no GPU.

use agent_core::readings::GpuReading;

/// The Apple GPUs, numbered from 0. T5 replaces this body; the caller and the series need no
/// change.
#[expect(
    clippy::unused_async,
    reason = "T5's reader runs ioreg, which is async; the caller already awaits"
)]
pub async fn read() -> Vec<GpuReading> {
    Vec::new()
}
