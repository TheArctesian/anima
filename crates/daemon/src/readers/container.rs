//! The collection in the container of a hosted hub (`anima-daemon run --hosted-hub DIR`): only
//! figures of that container (HMD-071, HMD-074). The memory and the processor come from the
//! control group of the container, the disk is the bytes of the data of the hub with no total
//! (HMD-072). It reads no `/proc` file, runs no command and reads no GPU: in a container those
//! can show the server. Parsing is `daemon_core::readings::container`; this only fetches bytes.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use daemon_core::readings::{HubStorage, Reading, container};

/// The control group of the container, under the root of the file system.
const CGROUP: &str = "sys/fs/cgroup";

/// How often the data folder is measured again: a walk of its files costs more than a read.
const MEASURE_EVERY: Duration = Duration::from_mins(1);

/// What the container reader keeps between two collections.
#[derive(Debug)]
pub struct Container {
    /// `/`; a test gives a folder that holds the files of a container and of a server.
    root: PathBuf,
    /// The folder of the data of the hub.
    data: PathBuf,
    /// The processors that the container may run on.
    processors: u64,
    /// The start of the clock of the processor share, and the processor time used then.
    start: Option<(Instant, u64)>,
    /// The bytes of the data and when they were measured.
    measured: Option<(Instant, u64)>,
}

impl Container {
    /// The reader of the container whose files are under `root`, whose hub keeps its data in
    /// `data`, and which may run on `processors` processors.
    #[must_use]
    pub const fn new(root: PathBuf, data: PathBuf, processors: u64) -> Self {
        Self {
            root,
            data,
            processors,
            start: None,
            measured: None,
        }
    }

    /// One collection at `now`: the memory, the processor and the data, and nothing else.
    pub async fn reading(&mut self, now: Instant) -> Reading {
        let group = self.root.join(CGROUP);
        let file = |name: &str| read(group.join(name));
        let (memory_max, memory_current, memory_stat, cpu_max, cpu_stat) = tokio::join!(
            file("memory.max"),
            file("memory.current"),
            file("memory.stat"),
            file("cpu.max"),
            file("cpu.stat"),
        );
        let stat = memory_stat.unwrap_or_default();
        let memory = memory_max
            .zip(memory_current)
            .and_then(|(max, current)| container::memory(&max, &current, &stat));
        let used = cpu_stat.as_deref().and_then(container::cpu_usage_micros);
        let allowed = container::allowed_processors(&cpu_max.unwrap_or_default(), self.processors);
        let cpu = used.map(|used| {
            let (since, at_start) = *self.start.get_or_insert((now, used));
            let elapsed = u64::try_from(now.duration_since(since).as_micros()).unwrap_or(u64::MAX);
            container::cpu_ticks(used.saturating_sub(at_start), elapsed, allowed)
        });
        Reading {
            cpu,
            memory,
            hub_storage: self.hub_storage(now).await,
            ..Reading::default()
        }
    }

    /// The bytes of the data folder, measured again when the last measure is a minute old.
    async fn hub_storage(&mut self, now: Instant) -> Option<HubStorage> {
        let due = self
            .measured
            .is_none_or(|(at, _)| now.duration_since(at) >= MEASURE_EVERY);
        if due {
            let folder = self.data.clone();
            let bytes = tokio::task::spawn_blocking(move || bytes_under(&folder)).await;
            self.measured = bytes.ok().flatten().map(|bytes| (now, bytes));
            if self.measured.is_none() {
                tracing::debug!("the data folder of the hub could not be measured");
            }
        }
        let (_, bytes) = self.measured?;
        Some(HubStorage {
            folder: self.data.to_str()?.to_owned(),
            bytes,
        })
    }
}

/// The bytes of a file; an empty answer for a file that cannot be read.
async fn read(path: PathBuf) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}

/// The sum of the lengths of the files under `folder`, as `du -sb` counts them. A link is not
/// followed, so nothing outside the folder counts; an entry that cannot be read is skipped.
/// `None` when the folder itself cannot be read.
fn bytes_under(folder: &Path) -> Option<u64> {
    let mut total = 0_u64;
    let mut folders = vec![folder.to_path_buf()];
    let mut first = true;
    while let Some(next) = folders.pop() {
        let entries = match std::fs::read_dir(&next) {
            Ok(entries) => entries,
            Err(_) if first => return None,
            Err(_) => continue,
        };
        first = false;
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                folders.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Some(total)
}
