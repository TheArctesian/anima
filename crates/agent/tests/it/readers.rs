//! The Linux readers on the machine that runs the tests: read only (MET-002).

use agent_core::points::samples;
use anima_agent::readers::{Platform, reading};
use chrono::Utc;

#[tokio::test]
async fn met_002_a_linux_reading_has_cpu_memory_load_uptime_and_disks() {
    let first = reading(Platform::Linux, Utc::now()).await;
    assert!(first.cpu.is_some());
    assert!(first.memory.is_some_and(|memory| memory.total_bytes > 0));
    assert!(first.load.is_some());
    assert!(first.uptime_seconds.is_some());
    assert!(!first.disks.is_empty(), "df -P -B1 listed no disk");
    assert!(first.interfaces.iter().any(|one| !one.name.is_empty()));
    let second = reading(Platform::Linux, Utc::now()).await;
    let made = samples(Some(&first), &second, std::time::Duration::from_secs(2));
    assert!(
        made.iter()
            .any(|sample| sample.kind.as_str() == "memory_percent")
    );
}
