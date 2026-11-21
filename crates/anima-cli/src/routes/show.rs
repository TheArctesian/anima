//! `anima route show <id>`: one public name with its proof, certificate and counts.

use super::documents::{RouteShow, route_show};
use super::shown;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The route's ID (`anima route list`).
    id: String,
}

impl ShowArgs {
    /// # Errors
    /// [`Failure::NotFound`] for an unknown route; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let variables = route_show::Variables {
            id: self.id.clone(),
        };
        let route = invocation
            .hub()?
            .run::<RouteShow>(variables)
            .await?
            .application_route
            .ok_or_else(|| Failure::NotFound {
                message: format!("No route has the ID {}.", self.id),
                hint: Some(Hint::run("anima route list")),
            })?;
        invocation.output.data(&route, shown::record)
    }
}
