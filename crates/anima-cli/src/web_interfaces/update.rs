//! `anima web update`: replaces a web interface's address, label and place to open. A flag
//! left out takes its K1 default, as at creation.

use super::documents::{WebUpdate, web_update};
use super::fields::WebFields;
use super::shown::Shown;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// The web interface's ID (`anima web list <device>`).
    id: String,
    #[command(flatten)]
    fields: WebFields,
}

impl UpdateArgs {
    /// Prints the web interface after the change.
    ///
    /// # Errors
    /// The hub's refusal, such as `NOT_FOUND` for an unknown ID.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = web_update::Variables {
            id: self.id,
            input: self.fields.typed()?,
        };
        let web = hub.run::<WebUpdate>(variables).await?.update_web_interface;
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
