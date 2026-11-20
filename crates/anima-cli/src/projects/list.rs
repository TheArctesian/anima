//! `anima project list`: the projects, by name, with pages (CMD-026).

use super::documents::{ProjectList, project_list};
use super::render;
use crate::connection_page;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::paging::{CommandLine, MAX_PAGE, fetch, note_next, page_size};

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only the projects whose repositories are all left out.
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
    #[command(flatten)]
    browse: Browse,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Order {
    Name,
    LastCommit,
    Created,
    CommitCount,
    Stars,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Browse {
    /// Order of projects across all pages.
    #[arg(long, value_enum)]
    pub order: Option<Order>,
    /// GitHub's primary language.
    #[arg(long)]
    pub language: Option<String>,
    /// Minimum repository size in KiB.
    #[arg(long)]
    pub min_size_kib: Option<i32>,
    /// Maximum repository size in KiB.
    #[arg(long)]
    pub max_size_kib: Option<i32>,
}

impl Browse {
    pub(crate) fn variables(&self) -> Option<project_list::ProjectBrowse> {
        if self.order.is_none()
            && self.language.is_none()
            && self.min_size_kib.is_none()
            && self.max_size_kib.is_none()
        {
            return None;
        }
        Some(project_list::ProjectBrowse {
            order: self.order.map(|order| match order {
                Order::Name => project_list::ProjectOrder::NAME,
                Order::LastCommit => project_list::ProjectOrder::LAST_COMMIT,
                Order::Created => project_list::ProjectOrder::CREATED,
                Order::CommitCount => project_list::ProjectOrder::COMMIT_COUNT,
                Order::Stars => project_list::ProjectOrder::STARS,
            }),
            language: self.language.clone(),
            min_size_kib: self.min_size_kib.map(i64::from),
            max_size_kib: self.max_size_kib.map(i64::from),
            group: None,
            ungrouped: None,
        })
    }
}

connection_page!(project_list::ProjectListProjects);

impl ListArgs {
    /// Prints the projects as a table or, with `--json`, as the K1 `ProjectConnection`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let command = CommandLine::new("anima project list", self.limit)
            .switch("--left-out", self.left_out)
            .choices("--order", &self.browse.order)
            .flag("--language", self.browse.language.as_deref())
            .flag(
                "--min-size-kib",
                self.browse
                    .min_size_kib
                    .map(|size| size.to_string())
                    .as_deref(),
            )
            .flag(
                "--max-size-kib",
                self.browse
                    .max_size_kib
                    .map(|size| size.to_string())
                    .as_deref(),
            );
        let hub = invocation.hub()?;
        let first = Some(page_size(self.limit, self.all, MAX_PAGE));
        let left_out = Some(self.left_out);
        let hub = &hub;
        let browse = &self.browse;
        let projects = fetch(self.after, self.all, move |after| async move {
            let variables = project_list::Variables {
                left_out,
                first,
                after,
                browse: browse.variables(),
            };
            Ok(hub.run::<ProjectList>(variables).await?.projects)
        })
        .await?;
        invocation.output.data(&projects, render::list)?;
        note_next(invocation, &projects, "projects", &command);
        Ok(())
    }
}
