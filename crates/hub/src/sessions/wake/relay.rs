//! The site's wake relay and the one command it runs for a wake (D-4, CHK-012): `wakeonlan`
//! and the MAC address the hub renders, nothing else.

use std::time::Duration;

use super::Error;
use crate::{
    devices::{Device, MacAddress},
    ssh::{FixedCommand, SshLink},
};

/// The program the relay runs. Debian and Ubuntu ship it in the `wakeonlan` package.
const PROGRAM: &str = "wakeonlan";
/// The longest `wakeonlan` may take on the relay. It sends one UDP broadcast and exits; ten
/// seconds leaves room for a slow relay and still answers the operator soon.
pub(super) const LIMIT: Duration = Duration::from_secs(10);

/// The relay that sends `device`'s packet: the site's `relay`, when there is one and it is not
/// the device itself (a device that is off cannot wake itself). Pure.
///
/// # Errors
/// [`Error::NoRelay`] or [`Error::RelayIsDevice`].
pub(super) fn chosen(device: &Device, relay: Option<Device>) -> Result<Device, Error> {
    let relay = relay.ok_or(Error::NoRelay)?;
    if relay.id == device.id {
        return Err(Error::RelayIsDevice);
    }
    Ok(relay)
}

/// Runs `wakeonlan <mac>` on the relay's `link` within [`LIMIT`]. Exit status 0 counts as
/// sent; the output is dropped unread (LOG-007).
///
/// # Errors
/// [`Error::Link`] when the command does not run or does not finish in time;
/// [`Error::NotSent`] for any other exit status, or none.
pub(super) async fn send(link: &mut SshLink, mac: MacAddress) -> Result<(), Error> {
    let command = FixedCommand::new(PROGRAM).mac(mac);
    let output = link.run(&command, LIMIT).await.map_err(Error::Link)?;
    if output.exit_status == Some(0) {
        Ok(())
    } else {
        Err(Error::NotSent)
    }
}
