//! The Linux collection: five `/proc` files and `df -P -B1` (MET-002).

use std::time::Duration;

use agent_core::readings::{Reading, df, linux};

use super::command::FixedCommand;

const DF: FixedCommand = FixedCommand {
    program: "df",
    args: &["-P", "-B1"],
    // A hung network mount must not stall the sampling.
    limit: Duration::from_millis(1500),
    keep_cut_output: false,
};

pub async fn reading() -> Reading {
    let (stat, meminfo, net, uptime, loadavg) = tokio::join!(
        proc("/proc/stat"),
        proc("/proc/meminfo"),
        proc("/proc/net/dev"),
        proc("/proc/uptime"),
        proc("/proc/loadavg"),
    );
    let disks = match DF.run().await {
        Ok(output) => df::disks(&output, 1),
        Err(error) => {
            tracing::debug!(%error, "df gave no reading");
            Vec::new()
        }
    };
    Reading {
        cpu: stat.and_then(|bytes| linux::cpu(&bytes)),
        memory: meminfo.and_then(|bytes| linux::memory(&bytes)),
        load: loadavg.and_then(|bytes| linux::load_average(&bytes)),
        uptime_seconds: uptime.and_then(|bytes| linux::uptime_seconds(&bytes)),
        disks,
        interfaces: net
            .map(|bytes| linux::interfaces(&bytes))
            .unwrap_or_default(),
    }
}

async fn proc(path: &str) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}
