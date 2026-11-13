//! `anima release list`: releases, newest first, in pages (CMD-026).

use super::documents::{ReleaseList, release_list};
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};
use crate::paging::{CommandLine, fetch, note_next, page_size};

/// The hub's largest page of releases.
const MAX_PAGE: i64 = 100;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Releases per page, 1 to 100.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

impl ListArgs {
    /// Prints the releases as a table or, with `--json`, as the K1 `ReleaseConnection`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let hub = &hub;
        let releases = fetch(self.after, self.all, move |after| async move {
            let variables = release_list::Variables { first, after };
            Ok(hub.run::<ReleaseList>(variables).await?.releases)
        })
        .await?;
        invocation.output.data(&releases, table)?;
        note_next(
            invocation,
            &releases,
            "releases shown",
            &CommandLine::new("anima release list", self.limit),
        );
        Ok(())
    }
}

connection_page!(release_list::ReleaseListReleases);

/// `LINUX_X86_64 PUBLISHED, DARWIN_UNIVERSAL FAILED`: one entry for each job.
pub fn job_summary<'a>(jobs: impl Iterator<Item = (String, &'a str)>) -> String {
    let parts: Vec<String> = jobs
        .map(|(target, state)| format!("{target} {state}"))
        .collect();
    if parts.is_empty() {
        "-".to_owned()
    } else {
        parts.join(", ")
    }
}

fn table(releases: &release_list::ReleaseListReleases) -> Table {
    let mut table = Table::new(vec!["VERSION", "COMMIT", "CREATED", "BY", "JOBS", "ID"]);
    for edge in &releases.edges {
        let release = &edge.node;
        let states: Vec<String> = release
            .jobs
            .iter()
            .map(|job| enum_name(&job.state))
            .collect();
        let jobs = job_summary(
            release
                .jobs
                .iter()
                .zip(&states)
                .map(|(job, state)| (enum_name(&job.target), state.as_str())),
        );
        table.row(vec![
            release.version.as_str().into(),
            release.commit.chars().take(12).collect::<String>().into(),
            release.created_at.as_str().into(),
            release.created_by.as_str().into(),
            Cell::new(&jobs),
            release.id.as_str().into(),
        ]);
    }
    table
}
