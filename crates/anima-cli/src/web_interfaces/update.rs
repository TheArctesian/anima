//! `anima web update`: changes a web interface's address, label or place to open. The flags are
//! laid over the current values, which are read first.

use super::current;
use super::documents::{WebUpdate, web_update};
use super::fields::WebChanges;
use super::shown::Shown;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// The web interface's ID (`anima web list <device>`).
    id: String,
    #[command(flatten)]
    changes: WebChanges,
}

impl UpdateArgs {
    /// Prints the web interface after the change.
    ///
    /// # Errors
    /// The hub's refusal, such as `NOT_FOUND` for an unknown ID.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.changes.is_empty() {
            return Err(Failure::Usage {
                message: "Nothing to change: give at least one flag.".to_owned(),
                hint: Some(Hint::run("anima web update --help")),
            });
        }
        let hub = invocation.hub()?;
        let current = current::find(&hub, &self.id).await?;
        let variables = web_update::Variables {
            input: self.changes.typed_over(current)?,
            id: self.id,
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
