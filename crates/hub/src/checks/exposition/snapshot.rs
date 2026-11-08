//! Reads one [`Snapshot`] in a fixed number of statements, however many checks and devices
//! there are (D-24, API-005), and applies the status rules that `Check.status` and
//! `Device.status` use, so the exposition never disagrees with the API (CHK-004, CHK-006).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::model::{CheckSample, DeviceLabels, DeviceSample, Snapshot, WINDOWS};
use crate::{
    checks::{Check, Error, from_devices, status, store, uptime},
    devices::{self, Device},
    ids::{CheckId, DeviceId},
    maintenance::{self, ActiveTargets},
    status::Status,
};

/// What [`take`] read, before the rules apply.
struct Reads {
    now: DateTime<Utc>,
    devices: Vec<Device>,
    active: ActiveTargets,
    standings: Vec<status::Standing>,
    checks: Vec<Check>,
    response_times: HashMap<CheckId, u32>,
    uptimes: [HashMap<CheckId, f64>; 3],
}

/// Every device and check as the exposition shows them at `now`, in eight statements however
/// many there are: devices, maintenance windows, standings, checks, newest heartbeats and one
/// uptime sum per window. Uptime comes from the aggregate tables only (CHK-011). The eight
/// reads are not one transaction, so a write between two of them can show in one and not the
/// other until the next build.
///
/// # Errors
/// `Database`, `Devices`, `Maintenance` or `Unreadable` when a read fails.
pub(super) async fn take(db: &PgPool, now: DateTime<Utc>) -> Result<Snapshot, Error> {
    let devices = devices::list(db).await.map_err(from_devices)?;
    let ids: Vec<DeviceId> = devices.iter().map(|device| device.id).collect();
    let active = maintenance::active_targets(db, now)
        .await
        .map_err(Error::Maintenance)?;
    let standings = store::standings(db, &ids).await?;
    let checks = store::by_devices(db, &ids).await?;
    let check_ids: Vec<CheckId> = checks.iter().map(|check| check.id).collect();
    let response_times = store::latest_heartbeats(db, &check_ids)
        .await?
        .into_iter()
        .filter_map(|(id, beat)| beat.response_time_ms.map(|ms| (id, ms)))
        .collect();
    let mut uptimes: [HashMap<CheckId, f64>; 3] = Default::default();
    for (ratios, (window, _)) in uptimes.iter_mut().zip(WINDOWS) {
        let span = (window.start(now), now);
        let tallies = store::check_tallies(db, &check_ids, window.resolution(), span).await?;
        ratios.extend(
            tallies
                .into_iter()
                .filter_map(|(id, tally)| uptime::ratio(tally).map(|ratio| (id, ratio))),
        );
    }
    let reads = Reads {
        now,
        devices,
        active,
        standings,
        checks,
        response_times,
        uptimes,
    };
    Ok(assemble(&reads))
}

fn labels(device: &Device) -> DeviceLabels {
    DeviceLabels {
        id: device.id,
        name: device.name.clone(),
        site: device.site,
    }
}

/// The samples: devices in name order, checks by device in that order, then by name. A
/// check's status is `status::of_check`, a device's is `status::by_device`, as in the API.
fn assemble(reads: &Reads) -> Snapshot {
    let ids: Vec<DeviceId> = reads.devices.iter().map(|device| device.id).collect();
    let shown = status::by_device(&ids, &reads.standings, &reads.devices, &reads.active);
    let mut by_device: HashMap<DeviceId, Vec<status::Standing>> = HashMap::new();
    for standing in &reads.standings {
        by_device
            .entry(standing.device)
            .or_default()
            .push(*standing);
    }
    let order: HashMap<DeviceId, (usize, &Device)> = reads
        .devices
        .iter()
        .enumerate()
        .map(|(index, device)| (device.id, (index, device)))
        .collect();
    let mut checks: Vec<(usize, CheckSample)> = reads
        .checks
        .iter()
        .filter_map(|check| {
            let &(index, device) = order.get(&check.device)?;
            let own = by_device.get(&device.id).map_or(&[][..], Vec::as_slice);
            let status = status::of_check(check.id, device, own, &reads.active);
            Some((index, sample(check, device, status, reads)))
        })
        .collect();
    // Stable: the store gives each device's checks in name order.
    checks.sort_by_key(|(index, _)| *index);
    let devices = reads
        .devices
        .iter()
        .map(|device| DeviceSample {
            device: labels(device),
            status: shown.get(&device.id).copied().unwrap_or(Status::Unknown),
        })
        .collect();
    Snapshot {
        checks: checks.into_iter().map(|(_, sample)| sample).collect(),
        devices,
    }
}

fn sample(check: &Check, device: &Device, status: Status, reads: &Reads) -> CheckSample {
    CheckSample {
        id: check.id,
        name: check.name.clone(),
        kind: check.kind(),
        device: labels(device),
        status,
        response_time_ms: reads.response_times.get(&check.id).copied(),
        uptime: reads
            .uptimes
            .each_ref()
            .map(|ratios| ratios.get(&check.id).copied()),
        certificate_days_left: check.certificate_days_left(reads.now),
        domain_days_left: check.domain_days_left(reads.now),
    }
}
