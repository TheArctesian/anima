//! The fixed memory overhead of each runtime and backend (FIT-003): compute buffers, scratch
//! space and the runtime itself. The defaults are estimates; measurements replace them.

const MIB: u64 = 1024 * 1024;

/// The overhead of one runtime on one backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverheadEntry {
    pub runtime: String,
    pub backend: String,
    pub bytes: u64,
}

/// The configuration of overheads, with a fallback for a pair it does not list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverheadTable {
    pub fallback_bytes: u64,
    pub entries: Vec<OverheadEntry>,
}

impl Default for OverheadTable {
    fn default() -> Self {
        let entry = |runtime: &str, backend: &str, mib: u64| OverheadEntry {
            runtime: runtime.to_owned(),
            backend: backend.to_owned(),
            bytes: mib * MIB,
        };
        Self {
            fallback_bytes: 1024 * MIB,
            entries: vec![
                entry("llama.cpp", "metal", 512),
                entry("llama.cpp", "cuda", 768),
                entry("llama.cpp", "vulkan", 768),
                entry("llama.cpp", "cpu", 256),
                entry("mlx", "metal", 512),
            ],
        }
    }
}

impl OverheadTable {
    /// The overhead for a runtime and backend, or the fallback when the pair is not listed.
    #[must_use]
    pub fn bytes_for(&self, runtime: &str, backend: &str) -> u64 {
        self.entries
            .iter()
            .find(|e| e.runtime == runtime && e.backend == backend)
            .map_or(self.fallback_bytes, |e| e.bytes)
    }

    /// Replace (or add) the overhead of a pair with a measured value.
    pub fn record_measurement(&mut self, runtime: &str, backend: &str, bytes: u64) {
        match self
            .entries
            .iter_mut()
            .find(|e| e.runtime == runtime && e.backend == backend)
        {
            Some(e) => e.bytes = bytes,
            None => self.entries.push(OverheadEntry {
                runtime: runtime.to_owned(),
                backend: backend.to_owned(),
                bytes,
            }),
        }
    }
}
