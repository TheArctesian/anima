//! The hub as a device of its own list (HMD-045): a hosted hub adds itself at its start, in
//! the site of the link, with no SSH, RDP or VNC.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use chrono::{DateTime, Utc};
use ipnet::{IpNet, Ipv6Net};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use super::{
    Error, FieldError,
    graphql::trail,
    model::{Device, NewDeviceInput, NewSiteInput, OperatingSystem},
    remote::RemoteForm,
    rules::{validate_new_device, validate_new_site},
    store,
};
use crate::{
    fleet::events::{Bus, ClientAddr, Operator, Trail},
    ids::{DeviceId, SiteId},
    scalars::Cidr,
};

/// The host of the hub's own device. The device is found by it, so a new name stays.
const HOST: &str = "localhost";
const KIND: &str = "hub";
const NAME: &str = "Hub";
/// How many names [`names`] gives.
const NAMES: usize = 10;
/// The site of the link, as the client makes it for the computers of the link (HMD-044).
const LINK_SITE: &str = "Link";
const LINK_SUBNET: Ipv6Net =
    Ipv6Net::new_assert(Ipv6Addr::new(0xfd41, 0x4e49, 0x4d41, 0, 0, 0, 0, 0), 48);

/// The names the hub's device can take, the first choice first: `Hub`, `Hub (hosted)`,
/// `Hub (hosted 2)` and so on.
pub(super) fn names() -> Vec<String> {
    let numbered = (2..NAMES).map(|number| format!("{NAME} (hosted {number})"));
    [NAME.to_owned(), format!("{NAME} (hosted)")]
        .into_iter()
        .chain(numbered)
        .collect()
}

/// The first of `names` that is not in `taken`. A device name is unique.
pub(super) fn free_name(names: Vec<String>, taken: &[String]) -> Option<String> {
    names.into_iter().find(|name| !taken.contains(name))
}

/// The hub's own work is written for no operator, from the hub's own address (LOG-014).
fn hub_trail(bus: &Bus, now: DateTime<Utc>) -> Trail<'_> {
    let client = ClientAddr(IpAddr::V4(Ipv4Addr::LOCALHOST));
    Trail::new(bus, (Operator::hub(), client), now)
}

/// The site of the link: the one that exists, else a new one with its audit entry.
async fn link_site(
    conn: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<SiteId, Error> {
    let subnet = IpNet::V6(LINK_SUBNET);
    if let Some(site) = store::site_for(conn, LINK_SITE, subnet).await? {
        return Ok(site.id);
    }
    let subnet = Cidr::new(subnet).map_err(|_| subnet_refused())?;
    let name = LINK_SITE.to_owned();
    let new = validate_new_site(&NewSiteInput { name, subnet }).map_err(Error::Validation)?;
    let site = store::create_site(conn, SiteId::from(Uuid::now_v7()), &new).await?;
    let entry = trail::site_added(&site);
    let written = hub_trail(bus, now).write(conn, entry).await;
    written.map_err(Error::Trail)?;
    Ok(site.id)
}

fn subnet_refused() -> Error {
    Error::Validation(vec![FieldError::new("subnet", "That subnet is not valid.")])
}

fn names_taken() -> Error {
    let message = "That name is already taken. Choose another.";
    Error::Validation(vec![FieldError::new("name", message)])
}

/// Makes sure that the hub is a device of its own list, in the caller's transaction (HMD-045).
/// A device with the host `localhost` is the hub's, whatever its name. When there is none, adds
/// one: the first free name of `Hub`, `Hub (hosted)`…, the kind `hub`, Linux, in the site
/// `Link`, with no SSH, RDP or VNC and no check, and writes its audit entry and event as the
/// hub. Returns the device and whether it was added now.
///
/// # Errors
/// `Validation` when every name is taken; `Trail` when the audit entry cannot be written.
pub async fn ensure_hub_device(
    conn: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<(Device, bool), Error> {
    let mut tx = conn.begin().await?;
    store::lock_hub_device(&mut tx).await?;
    if let Some(device) = store::device_at_host(&mut tx, HOST).await? {
        tx.commit().await?;
        return Ok((device, false));
    }
    let site = link_site(&mut tx, bus, now).await?;
    let taken = store::names_in_use(&mut tx, &names()).await?;
    let form = NewDeviceInput {
        site,
        name: free_name(names(), &taken).ok_or_else(names_taken)?,
        host: HOST.to_owned(),
        kind: KIND.to_owned(),
        os: OperatingSystem::Linux,
        tags: Vec::new(),
        ssh: None,
        remote: RemoteForm::default(),
    };
    let new = validate_new_device(&form).map_err(Error::Validation)?;
    let id = DeviceId::from(Uuid::now_v7());
    let device = store::create_device(&mut tx, id, now, &new).await?;
    let entry = trail::device_added(&device, &new, false);
    let written = hub_trail(bus, now).write(&mut tx, entry).await;
    written.map_err(Error::Trail)?;
    tx.commit().await?;
    Ok((device, true))
}
