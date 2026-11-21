//! `anima route add <name>`: makes an application public under a name, and prints the TXT
//! record that proves the name. A login token only, with a second factor on the account
//! (SF-NEW-routepublish).

use super::documents::{RouteAdd, route_add};
use super::shown;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// The DNS name, such as `photos.example.com`.
    name: String,
    /// The application's ID.
    #[arg(long)]
    application: String,
    /// The device's ID.
    #[arg(long)]
    device: String,
    /// The port that the applications program recorded for the application.
    #[arg(long)]
    port: i64,
    /// The application holds the socket of the engine: the hub refuses it.
    #[arg(long)]
    holds_engine_socket: bool,
}

impl AddArgs {
    /// Asks for confirmation (`--yes` skips it), adds the name and prints the route.
    ///
    /// # Errors
    /// [`Failure::Usage`] when it cannot ask; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let question = format!(
            "Make the application public on the internet as {}? Its page is then open to each person.",
            self.name
        );
        invocation.prompter.confirm(&question)?;
        let input = route_add::ApplicationRouteInput {
            name: self.name,
            application_id: self.application,
            device_id: self.device,
            port: self.port,
            holds_engine_socket: self.holds_engine_socket,
        };
        let added = invocation
            .hub()?
            .run::<RouteAdd>(route_add::Variables { input })
            .await?
            .add_application_route;
        invocation.output.data(&added.route, shown::record)?;
        if added.same_site_as_hub {
            invocation.output.note(
                "The name shares its domain with the hub: a page of the application can send \
                 requests to the hub with its cookies.",
            );
        }
        Ok(())
    }
}
