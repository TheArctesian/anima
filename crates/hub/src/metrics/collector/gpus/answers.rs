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
    /// The GPUs it reported; none when the tool or files are missing or it found no GPU.
    Gpus(Vec<GpuReading>),
    /// The tool ran and failed: an exit status other than 0 and 127, so its output is not
    /// read. It may not read the GPU, or its driver broke. Discovery decides what this means
    /// from the GPUs known before (D-30).
    Failed,
    /// The read did not finish, so the hub cannot tell: it timed out, the device refused it,
    /// the link was lost, or the output was cut or too long.
    Unknown,
}

/// The exit status of a command the shell, or `timeout`, did not find.
const MISSING: u32 = 127;

/// The answer of `vendor`'s read that left `output`. Exit status 127 means no GPU of that
/// vendor and any other non-zero status a failed read, except for AMD: `grep` exits 2 when
/// one of its globs matched no file, and still prints the files it read.
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
        _ if status == MISSING => Vec::new(),
        _ if status != 0 => return Answer::Failed,
        GpuVendor::Nvidia => nvidia::gpus(stdout),
        GpuVendor::Intel => intel::gpu(stdout).into_iter().collect(),
        GpuVendor::Apple => apple::gpus(stdout),
    };
    Answer::Gpus(gpus)
}
