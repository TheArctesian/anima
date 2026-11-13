//! `anima web create`: a web page of a device, such as its admin console.

use super::documents::{WebCreate, web_create};
use super::fields::WebFields;
use super::shown::Shown;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The device's ID or its exact name.
    device: String,
    #[command(flatten)]
    fields: WebFields,
}

impl CreateArgs {
    /// Prints the new web interface.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal, such as `VALIDATION`
    /// for a URL it does not accept.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = web_create::Variables {
            device_id: resolve(&hub, &self.device).await?,
            input: self.fields.typed()?,
        };
        let web = hub.run::<WebCreate>(variables).await?.create_web_interface;
        invocation.output.data(&web, |web| {
            Shown {
                id: &web.id,
                label: &web.label,
                url: &web.url,
                open_in: &enum_name(&web.open_in),
                pinned_fingerprint: web.pinned_fingerprint_sha256.as_deref(),
            }
            .table()
        })
    }
}
