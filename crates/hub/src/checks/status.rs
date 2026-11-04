//! The status a check and a device show: pause, groups and maintenance (CHK-004, CHK-006).
//! Pure: the stored state and the maintenance in force come in as parameters.

use std::collections::HashMap;

use crate::{
    devices::Device,
    ids::{CheckId, DeviceId},
    maintenance::ActiveTargets,
    status::{Status, worst},
};

/// What the status rules need to know about one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Standing {
    pub(super) id: CheckId,
    pub(super) device: DeviceId,
    pub(super) parent: Option<CheckId>,
    pub(super) group: bool,
    pub(super) paused: bool,
    /// The stored status of the check's last result.
    pub(super) status: Status,
}

/// The maintenance that applies to one device at one instant.
#[derive(Clone, Copy, Debug)]
pub(super) struct Cover<'a> {
    /// The device's manual maintenance flag (CHK-006).
    pub(super) manual: bool,
    pub(super) active: &'a ActiveTargets,
    pub(super) device: DeviceId,
    pub(super) tags: &'a [String],
}

impl Cover<'_> {
    /// Whether the manual flag or an active window covers the check (CHK-006).
    pub(super) fn covers(&self, check: CheckId) -> bool {
        self.manual || self.active.covers(self.device, Some(check), self.tags)
    }
}

/// Down becomes Maintenance under maintenance; every other status stays (CHK-006).
#[must_use]
pub(super) const fn maintained(status: Status, covered: bool) -> Status {
    match status {
        Status::Down if covered => Status::Maintenance,
        other => other,
    }
}

/// The status one check shows. A paused check is Unknown. A group is the worst of its
/// children that are not paused, or Unknown with none. Maintenance covers a check when
/// `covered` holds for the check itself or for its group.
pub(super) fn shown(
    check: &Standing,
    all: &[Standing],
    covered: &dyn Fn(CheckId) -> bool,
) -> Status {
    if check.paused {
        return Status::Unknown;
    }
    if check.group {
        return all
            .iter()
            .filter(|child| child.parent == Some(check.id) && !child.paused)
            .map(|child| shown(child, all, covered))
            .reduce(worst)
            .unwrap_or(Status::Unknown);
    }
    let under = covered(check.id) || check.parent.is_some_and(covered);
    maintained(check.status, under)
}

/// The worst status of the device's checks that are not paused and are not groups (a group
/// adds nothing beyond its children), or Unknown with none (CHK-004).
pub(super) fn device_status(checks: &[Standing], covered: &dyn Fn(CheckId) -> bool) -> Status {
    checks
        .iter()
        .filter(|check| !check.group && !check.paused)
        .map(|check| shown(check, checks, covered))
        .reduce(worst)
        .unwrap_or(Status::Unknown)
}

/// The status of each requested device. A device that `known` does not hold has no manual
/// flag and no tags.
pub(super) fn by_device(
    requested: &[DeviceId],
    checks: &[Standing],
    known: &[Device],
    active: &ActiveTargets,
) -> HashMap<DeviceId, Status> {
    let devices: HashMap<DeviceId, &Device> = known.iter().map(|d| (d.id, d)).collect();
    let mut grouped: HashMap<DeviceId, Vec<Standing>> = HashMap::new();
    for check in checks {
        grouped.entry(check.device).or_default().push(*check);
    }
    requested
        .iter()
        .map(|&id| {
            let device = devices.get(&id);
            let cover = Cover {
                manual: device.is_some_and(|d| d.maintenance),
                active,
                device: id,
                tags: device.map_or(&[], |d| d.tags.as_slice()),
            };
            let own = grouped.get(&id).map_or(&[][..], Vec::as_slice);
            (id, device_status(own, &|check| cover.covers(check)))
        })
        .collect()
}
