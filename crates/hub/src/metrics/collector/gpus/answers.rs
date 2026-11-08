//! What each GPU vendor's output says, read by the agent's readers (MET-016, MET-007, D-30).
//! Pure.

use agent_core::readings::{
    GpuReading,
    gpu::{apple, intel, nvidia},
};

use super::sysfs;
use crate::{metrics::GpuVendor, ssh::CommandOutput};

/// The most bytes of one GPU tool's output the hub reads. Honest output is a few kilobytes: a
/// line per NVIDIA GPU, three short lines per AMD card, a few `intel_gpu_top` samples, one
/// registry entry per accelerator. Longer output is not read, so a device cannot make the
/// readers build large values from the 4 MiB the link keeps.
pub(in crate::metrics::collector) const OUTPUT_LIMIT: usize = 256 * 1024;

/// What one vendor's read told the hub in one collection.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) enum Answer {
    /// The GPUs it reported; none when the tool or files are missing, the tool may not read
    /// the GPU, or it found no GPU.
    Gpus(Vec<GpuReading>),
    /// The read did not finish, so the hub cannot tell: it timed out, the device refused it,
    /// the link was lost, or the output was cut or too long.
    Unknown,
}

/// The answer of `vendor`'s read that left `output`. A non-zero exit status means no GPU of
/// that vendor, except for AMD: `grep` exits 2 when one of its globs matched no file, and
/// still prints the files it read.
pub(in crate::metrics::collector) fn answer(vendor: GpuVendor, output: &CommandOutput) -> Answer {
    let Some(status) = output.exit_status else {
        return Answer::Unknown;
    };
    if output.cut || output.stdout.len() > OUTPUT_LIMIT {
        return Answer::Unknown;
    }
    let stdout = output.stdout.as_slice();
    let gpus = match vendor {
        GpuVendor::Amd => sysfs::cards(stdout),
        _ if status != 0 => Vec::new(),
        GpuVendor::Nvidia => nvidia::gpus(stdout),
        GpuVendor::Intel => intel::gpu(stdout).into_iter().collect(),
        GpuVendor::Apple => apple::gpus(stdout),
    };
    Answer::Gpus(gpus)
}
