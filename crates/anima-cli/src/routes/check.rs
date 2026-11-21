//! `anima route check <id>`: reads the proof of the name once. One failed read changes nothing.

use super::documents::{RouteCheck, route_check};
use super::shown;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// The route's ID (`anima route list`).
    id: String,
}

impl CheckArgs {
    /// # Errors
    /// The hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let route = invocation
            .hub()?
            .run::<RouteCheck>(route_check::Variables { id: self.id })
            .await?
            .check_application_route;
        invocation.output.data(&route, shown::record)
    }
}
