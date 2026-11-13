//! The web interface that `web update` changes, as the hub has it now. K1 has no query by a web
//! interface's ID, so the devices are read in pages until one holds it.

use super::documents::{WebCurrent, web_current};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::output::text::enum_name;

/// The hub's largest page of devices.
const PAGE: i64 = 200;

/// A web interface's address, label and place to open, in K1 input form.
#[derive(Debug)]
pub struct Current {
    pub url: String,
    pub label: String,
    pub open_in: String,
}

/// The web interface with `id`.
///
/// # Errors
/// [`Failure::NotFound`] when no device holds it; the hub's refusal.
pub async fn find(hub: &Hub, id: &str) -> Result<Current, Failure> {
    let mut after = None;
    loop {
        let variables = web_current::Variables {
            first: Some(PAGE),
            after: after.take(),
        };
        let devices = hub.run::<WebCurrent>(variables).await?.devices;
        let found = devices
            .edges
            .iter()
            .flat_map(|edge| &edge.node.web_interfaces)
            .find(|web| web.id == id);
        if let Some(web) = found {
            return Ok(Current {
                url: web.url.clone(),
                label: web.label.clone(),
                open_in: enum_name(&web.open_in),
            });
        }
        if !devices.page_info.has_next_page {
            return Err(Failure::NotFound {
                message: format!("No web interface has the ID {id}."),
                hint: Some(Hint::run("anima web list <device>")),
            });
        }
        after = devices.page_info.end_cursor;
    }
}
