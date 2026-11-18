use serde::Serialize;

use super::name;
use crate::this_machine::OperatingSystem;

/// This computer as the hosted hub lists it (HMD-044), so the client can find its link device
/// and add it to the devices. None of it is secret.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CloudDevice {
    /// The name the hub gave the link device of this computer: what `cloud_hub_access` sent.
    name: String,
    /// The computer name as the OS gives it; empty when the OS gives none.
    computer: String,
    os: OperatingSystem,
}

/// The cloud device of the computer with the name `computer` that runs `os`.
pub(super) fn cloud_device(computer: &str, os: OperatingSystem) -> CloudDevice {
    CloudDevice {
        name: name::device_name(computer),
        computer: computer.to_owned(),
        os,
    }
}
