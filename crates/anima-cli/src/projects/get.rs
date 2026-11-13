//! `anima project get <slug>`.

use super::documents::{ProjectGet, project_get};
use super::render;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The project's slug.
    slug: String,
}

impl GetArgs {
    /// Prints the project or, with `--json`, the K1 `Project`.
    ///
    /// # Errors
    /// [`Failure::NotFound`] when no project has the slug; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<ProjectGet>(project_get::Variables {
                slug: self.slug.clone(),
            })
            .await?;
        let project = answer.project.ok_or_else(|| Failure::NotFound {
            message: format!("There is no project with the slug {}.", self.slug),
            hint: Some(Hint::run("anima project list")),
        })?;
        invocation.output.data(&project, render::record)
    }
}
