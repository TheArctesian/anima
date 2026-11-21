//! Reads the hardware values of this computer (CAP-001 to CAP-005, CAP-032): fixed commands and
//! fixed files only, nothing written, and hands the bytes to `daemon_core::hardware`, which
//! reads them. This is the I/O edge.

#[cfg(unix)]
mod linux;
#[cfg(unix)]
mod macos;

use daemon_core::hardware::Hardware;

use super::Platform;
#[cfg(unix)]
use super::command::FixedCommand;

/// The free disk space is sent in whole GiB, so a few files written do not announce it again.
#[cfg(unix)]
const GIB: u64 = 1024 * 1024 * 1024;

/// The hardware values of this computer; `None` when its memory cannot be read, so nothing is
/// sent (the hub keeps what it has). Windows sends none yet.
pub async fn hardware(platform: Platform) -> Option<Hardware> {
    match platform {
        #[cfg(unix)]
        Platform::Macos => macos::read().await,
        #[cfg(unix)]
        Platform::Linux => linux::read().await,
        #[cfg(windows)]
        Platform::Linux | Platform::Macos | Platform::Windows => None,
    }
}

/// The free space of the file system of the daemon's home folder (`/` without one), in whole
/// GiB.
#[cfg(unix)]
fn free_disk_bytes() -> Option<u64> {
    let home = std::env::var_os("HOME").unwrap_or_else(|| "/".into());
    let found = rustix::fs::statvfs(std::path::Path::new(&home)).ok()?;
    let bytes = found.f_bavail.saturating_mul(found.f_frsize);
    Some(bytes / GIB * GIB)
}

/// The fixed CPU features that model runtimes use, which this processor has.
#[must_use]
pub fn cpu_features() -> Vec<String> {
    let mut found: Vec<&str> = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::is_x86_feature_detected as has;
        let named = [
            ("sse4.2", has!("sse4.2")),
            ("avx", has!("avx")),
            ("avx2", has!("avx2")),
            ("fma", has!("fma")),
            ("f16c", has!("f16c")),
            ("avx512f", has!("avx512f")),
            ("avx512bw", has!("avx512bw")),
            ("avx512vnni", has!("avx512vnni")),
        ];
        found.extend(named.iter().filter(|(_, on)| *on).map(|(name, _)| *name));
    }
    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::is_aarch64_feature_detected as has;
        let named = [
            ("neon", has!("neon")),
            ("dotprod", has!("dotprod")),
            ("fp16", has!("fp16")),
            ("i8mm", has!("i8mm")),
            ("bf16", has!("bf16")),
            ("sve", has!("sve")),
            ("sve2", has!("sve2")),
        ];
        found.extend(named.iter().filter(|(_, on)| *on).map(|(name, _)| *name));
    }
    let mut features: Vec<String> = found.into_iter().map(str::to_owned).collect();
    features.sort_unstable();
    features
}

/// What `command` printed; `None` when it did not run or failed.
#[cfg(unix)]
async fn output(command: &FixedCommand) -> Option<Vec<u8>> {
    command
        .run()
        .await
        .inspect_err(|error| tracing::debug!(%error, "a hardware reader gave no value"))
        .ok()
}
