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

    use super::*;
    use crate::sampling::gpu::{announce, values};

    const WITH_MEMORY: &[u8] = br#"+-o AGXAcceleratorG14X  <class AGXAcceleratorG14X, id 0x100000a1e>
    {
      "model" = "Apple M2 Pro"
      "PerformanceStatistics" = {"Device Utilization %"=37,"In use system memory"=1073741824,"Alloc system memory"=2147483648}
    }
"#;

    const NO_MEMORY: &[u8] = br#"+-o AGXAcceleratorG14X  <class AGXAcceleratorG14X, id 0x100000a1e>
    {
      "model" = "Apple M2 Pro"
      "PerformanceStatistics" = {"Device Utilization %"=5}
    }
"#;

    fn kinds(gpus: &[GpuReading]) -> Vec<(&'static str, f64)> {
        values(gpus)
            .into_iter()
            .map(|(key, value)| (key.kind().as_str(), value))
            .collect()
    }

    #[test]
    fn met_006_ioreg_runs_the_accelerator_class_by_absolute_path_with_a_time_limit() {
        assert_eq!(IOREG.program, "/usr/sbin/ioreg");
        assert_eq!(IOREG.args, ["-r", "-d", "1", "-c", "IOAccelerator"]);
        assert!(IOREG.limit <= Duration::from_secs(2));
    }

    #[test]
    fn met_007_use_and_in_use_memory_become_gpu_percent_and_gpu_memory_bytes() {
        let gpus = readings(Ok(WITH_MEMORY.to_vec()));
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Apple);
        assert_eq!(gpus[0].vram, None, "never VRAM");
        assert_eq!(
            kinds(&gpus),
            [("gpu_percent", 37.0), ("gpu_memory_bytes", 1_073_741_824.0)]
        );
    }

    #[test]
    fn met_006_the_apple_gpu_is_announced_with_vendor_apple_and_no_vram() {
        let gpus = readings(Ok(WITH_MEMORY.to_vec()));
        let announced = announce(&gpus);
        assert_eq!(announced.len(), 1);
        assert_eq!(announced[0].name, "Apple M2 Pro");
        assert!(!announced[0].has_vram);
    }

    #[test]
    fn met_007_a_gpu_with_no_memory_figure_gives_use_only() {
        let gpus = readings(Ok(NO_MEMORY.to_vec()));
        assert_eq!(gpus.len(), 1);
        assert_eq!(kinds(&gpus), [("gpu_percent", 5.0)]);
    }

    #[test]
    fn met_006_a_failed_run_an_empty_answer_and_a_mac_without_a_gpu_give_no_gpus() {
        let failed = Err(CommandError::Failed { program: "ioreg" });
        assert!(readings(failed).is_empty());
        let missing = Err(CommandError::Timeout { program: "ioreg" });
        assert!(readings(missing).is_empty());
        assert!(readings(Ok(Vec::new())).is_empty());
        assert!(readings(Ok(b"\n".to_vec())).is_empty());
    }
}
