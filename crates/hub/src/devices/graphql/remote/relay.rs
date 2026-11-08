//! `setSiteWakeRelay`: which device sends Wake-on-LAN packets at a site (RA-031, D-7, LOG-005).

use async_graphql::{Context, ID, Result};

use crate::{
    api_errors::{begin, commit, invalid},
    devices::{
        FieldError,
        graphql::{
            credential::device_arg,
            error::to_graphql,
            trail::{self, Trail},
        },
        model::Site,
        rules::wake_relay,
        store,
    },
    ids::SiteId,
};

fn site_arg(id: &ID) -> Result<SiteId, FieldError> {
    id.parse()
        .map_err(|_| FieldError::new("siteId", "That site id is not valid."))
}

/// Sets the site's wake relay to the device, or clears it when `device` is `None`, in one
/// transaction with its audit entry and event. A call that changes nothing writes nothing.
pub(in crate::devices::graphql) async fn set_wake_relay(
    ctx: &Context<'_>,
    site: &ID,
    device: Option<&ID>,
) -> Result<Site> {
    let (site, device) = match (site_arg(site), device.map(device_arg).transpose()) {
        (Ok(site), Ok(device)) => (site, device),
        (site, device) => {
            let bad: Vec<_> = site.err().into_iter().chain(device.err()).collect();
            return Err(invalid(&bad));
        }
    };
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    // The device row before the site row: the order a device update or delete takes them in.
    let candidate = match device {
        Some(id) => Some(store::share_device(&mut tx, id).await.map_err(to_graphql)?),
        None => None,
    };
    let current = store::lock_site(&mut tx, site).await.map_err(to_graphql)?;
    let relay = candidate
        .as_ref()
        .map(|found| wake_relay(found.as_ref(), site))
        .transpose()
        .map_err(|error| invalid(&[error]))?;
    let relay_id = relay.map(|relay| relay.id);
    if current.wake_relay == relay_id {
        // Nothing to write: the transaction rolls back when it is dropped.
        return Ok(current);
    }
    let changed = store::set_wake_relay(&mut tx, site, relay_id)
        .await
        .map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::wake_relay_changed(&changed, relay))
        .await?;
    commit(tx).await?;
    Ok(changed)
}
