//! The Linux collection: six `/proc` files, `df -P -B1`, `lsblk -J -b` and the power supplies of
//! `/sys/class/power_supply` (MET-002, MET-027, MET-032).

use std::time::Duration;

use daemon_core::readings::{
    Battery, Drive, Reading,
    batteries::{self, POWER_SUPPLY, SUPPLY_FILES},
    df,
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
        hub_storage: None,
        batteries: battery_list().await,
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

/// The batteries among the power supplies; `None` when the folder cannot be listed. A file
/// that cannot be read is left out, so its value is unknown.
async fn battery_list() -> Option<Vec<Battery>> {
    let mut entries = match tokio::fs::read_dir(POWER_SUPPLY).await {
        Ok(entries) => entries,
        Err(error) => {
            tracing::debug!(%error, "the power supplies gave no batteries");
            return None;
        }
    };
    let mut found = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let mut texts = Vec::new();
        for file in SUPPLY_FILES {
            if let Ok(text) = tokio::fs::read_to_string(entry.path().join(file)).await {
                texts.push((file, text));
            }
        }
        let files: Vec<(&str, &str)> = texts.iter().map(|(f, t)| (*f, t.as_str())).collect();
        found.extend(batteries::linux_battery(&name, &files));
    }
    Some(batteries::batteries(found))
}

async fn proc(path: &str) -> Option<Vec<u8>> {
    tokio::fs::read(path).await.ok()
}
