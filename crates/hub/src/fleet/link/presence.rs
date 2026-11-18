//! Which devices of the link are connected now (HMD-047): a device of the link keeps one
//! stream open to the link of the hub and says `here` on it each 15 seconds, and the link
//! gives the time of the last one in `devices.list`.

use std::{net::IpAddr, path::Path};

use chrono::{DateTime, Utc};

use super::{model::ListedDevice, socket::Control, wire::Request};

/// One device of the link and when the link last saw it connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presence {
    /// The link address of the device: the host of its device in the hub.
    pub address: IpAddr,
    /// `None` when the device did not connect since the link started, or its link program
    /// does not say that it is connected.
    pub seen: Option<DateTime<Utc>>,
}

/// Asks the link at the control socket `control` for its devices.
///
/// # Errors
/// The fault word of the failed call, for the log: the link does not run, did not answer in
/// time, or its socket or its reply cannot be used (LOG-007).
pub async fn presence(control: &Path) -> Result<Vec<Presence>, &'static str> {
    let listed: Vec<ListedDevice> = Control::of_hub(control.to_owned())
        .call(Request::DevicesList)
        .await
        .map_err(|error| error.fault())?;
    let presence = listed.into_iter().map(|device| Presence {
        address: device.address,
        seen: device.seen,
    });
    Ok(presence.collect())
}
