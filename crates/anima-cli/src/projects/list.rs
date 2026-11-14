//! `anima project list`: the projects, newest read first, with pages (CMD-026).

use super::documents::{ProjectList, project_list};
use super::render;
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only the projects that have a repository you left out.
    #[arg(long)]
    left_out: bool,
    /// Projects per page, 1 to 200.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

connection_page!(project_list::ProjectListProjects);

impl ListArgs {
    /// Prints the projects as a table or, with `--json`, as the K1 `ProjectConnection`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command =
            CommandLine::new("anima project list", self.limit).switch("--left-out", self.left_out);
        let hub = invocation.hub()?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let left_out = Some(self.left_out);
        let hub = &hub;
        let projects = fetch(self.after, self.all, move |after| async move {
            let variables = project_list::Variables {
                left_out,
                first,
                after,
            };
            Ok(hub.run::<ProjectList>(variables).await?.projects)
        })
        .await?;
        invocation.output.data(&projects, render::list)?;
        note_next(invocation, &projects, "projects", &command);
        Ok(())
    }
}
