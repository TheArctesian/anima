//! The Apple GPU reader: one run of `ioreg -r -d 1 -c IOAccelerator` per 2 s sample, read by
//! `agent_core::readings::gpu::apple` (MET-006, MET-007, MET-016, R-092 D-4). No `unsafe`.
//!
//! `ioreg` runs on any Unix, so nothing here is behind `cfg(target_os = "macos")`; only a Mac
//! answers with an `IOAccelerator` entry. `IOReport` GPU power stays out (issue #1).

use std::time::Duration;

use agent_core::readings::GpuReading;
use agent_core::readings::gpu::apple;

use crate::readers::command::{CommandError, FixedCommand};

const IOREG: FixedCommand = FixedCommand {
    program: "/usr/sbin/ioreg",
    args: &["-r", "-d", "1", "-c", "IOAccelerator"],
    limit: Duration::from_secs(2),
    keep_cut_output: false,
};

/// The Apple GPUs, numbered from 0. A failed run gives none.
pub async fn read() -> Vec<GpuReading> {
    let answer = IOREG.run().await;
    if let Err(error) = &answer {
        tracing::debug!(%error, "ioreg gave no Apple GPU reading");
    }
    readings(answer)
}

/// The GPUs in one `ioreg` answer: none when the run failed or printed no accelerator. A GPU
/// whose entry has no memory figure is kept, with no `shared_memory_bytes` (MET-007: absent,
/// never zero, never VRAM).
fn readings(answer: Result<Vec<u8>, CommandError>) -> Vec<GpuReading> {
    answer.map(|bytes| apple::gpus(&bytes)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use agent_core::readings::GpuVendor;
    use agent_core::readings::captures::IOREG_ACCELERATOR;

    use super::*;
    use crate::sampling::gpu::{announce, values};

    fn kinds(gpus: &[GpuReading]) -> Vec<(&'static str, f64)> {
        values(gpus)
            .into_iter()
            .map(|(key, value)| (key.kind().as_str(), value))
            .collect()
    }

    #[test]
    fn met_006_ioreg_runs_with_a_time_limit_of_at_most_two_seconds() {
        assert!(IOREG.limit <= Duration::from_secs(2));
    }

    #[test]
    fn met_006_both_accelerators_are_read_with_their_numbering_use_and_in_use_memory() {
        let gpus = readings(Ok(IOREG_ACCELERATOR.to_vec()));
        assert_eq!(gpus.len(), 2);
        assert_eq!((gpus[0].index, gpus[1].index), (0, 1));
        assert!(gpus.iter().all(|gpu| gpu.vendor == GpuVendor::Apple));
        assert_eq!(gpus[0].busy_percent, Some(17.0));
        assert_eq!(gpus[0].shared_memory_bytes, Some(987_654_321));
        assert_eq!(gpus[1].busy_percent, Some(3.0));
        assert_eq!(gpus[1].shared_memory_bytes, Some(1000));
    }

    #[test]
    fn met_007_use_and_in_use_memory_become_gpu_percent_and_gpu_memory_bytes_never_vram() {
        let gpus = readings(Ok(IOREG_ACCELERATOR.to_vec()));
        assert!(gpus.iter().all(|gpu| gpu.vram.is_none()));
        assert_eq!(
            kinds(&gpus),
            [
                ("gpu_percent", 17.0),
                ("gpu_memory_bytes", 987_654_321.0),
                ("gpu_percent", 3.0),
                ("gpu_memory_bytes", 1000.0),
            ]
        );
    }

    #[test]
    fn met_006_the_apple_gpus_are_announced_with_vendor_apple_and_no_vram() {
        let gpus = readings(Ok(IOREG_ACCELERATOR.to_vec()));
        let announced = announce(&gpus);
        assert_eq!(announced.len(), 2);
        assert!(announced.iter().all(|gpu| gpu.name == "Apple M2 Pro"));
        assert!(announced.iter().all(|gpu| !gpu.has_vram));
    }

    #[test]
    fn met_006_a_failed_run_a_timed_out_run_and_an_empty_answer_give_no_gpus() {
        let failed = Err(CommandError::Failed { program: "ioreg" });
        assert_eq!(readings(failed).len(), 0);
        let timed_out = Err(CommandError::Timeout { program: "ioreg" });
        assert_eq!(readings(timed_out).len(), 0);
        assert_eq!(readings(Ok(Vec::new())).len(), 0);
        assert_eq!(readings(Ok(b"\n".to_vec())).len(), 0);
    }
}
