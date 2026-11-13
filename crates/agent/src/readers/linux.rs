//! The Linux collection: six `/proc` files, `df -P -B1` and `lsblk -J -b` (MET-002, MET-027).

use std::time::Duration;

use agent_core::readings::{
    Drive, Reading, df,
    drives::{self, LSBLK_ARGS, LSBLK_OLD_ARGS},
    linux,
};

use super::command::FixedCommand;

/// A hung network mount must not stall the sampling.
const QUICK: Duration = Duration::from_millis(1500);

const DF: FixedCommand = FixedCommand {
    program: "df",
    args: &["-P", "-B1"],
    limit: QUICK,
    keep_cut_output: false,
};

const LSBLK: FixedCommand = FixedCommand {
    program: "lsblk",
    args: &LSBLK_ARGS,
    limit: QUICK,
    keep_cut_output: false,
};

/// For util-linux before 2.37, which refuses `MOUNTPOINTS` (MET-027).
const LSBLK_OLD: FixedCommand = FixedCommand {
    program: "lsblk",
    args: &LSBLK_OLD_ARGS,
    limit: QUICK,
    keep_cut_output: false,
};

pub async fn reading() -> Reading {
    let (stat, meminfo, net, uptime, loadavg, diskstats) = tokio::join!(
        proc("/proc/stat"),
        proc("/proc/meminfo"),
        proc("/proc/net/dev"),
        proc("/proc/uptime"),
        proc("/proc/loadavg"),
        proc("/proc/diskstats"),
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
        interfaces: net.map_or_default(|bytes| linux::interfaces(&bytes)),
        drives: drive_list().await,
        drive_counters: diskstats.map_or_default(|bytes| linux::drive_counters(&bytes)),
    }
}

/// The drives `lsblk` lists, with the older column name when the newer one is refused; `None`
/// when neither form gives a readable list.
async fn drive_list() -> Option<Vec<Drive>> {
    let output = match LSBLK.run().await {
        Ok(output) => Ok(output),
        Err(_) => LSBLK_OLD.run().await,
    };
    match output {
        Ok(output) => drives::drives(&output),
        Err(error) => {
            tracing::debug!(%error, "lsblk gave no drives");
            None
        }
    }
}

async fn proc(path: &str) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}
