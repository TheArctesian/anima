//! `anima release log <job id>`: the stored log of a build job, oldest line first (BLD-048).

use std::io::Write as _;

use super::documents::{BuildJobLog, build_job_log};
use super::follow;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::clean;
use crate::paging::{CommandLine, Page, fetch, note_next, page_size};

/// The hub's largest page of log lines.
const MAX_PAGE: i64 = 1000;

type Lines = Vec<build_job_log::BuildJobLogBuildJobLog>;

#[derive(Debug, clap::Args)]
pub struct LogArgs {
    /// The build job's ID.
    job: String,
    /// Lines per page, 1 to 1000.
    #[arg(long, default_value_t = 500, conflicts_with = "follow", value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this line sequence (from the previous page).
    #[arg(long, conflicts_with_all = ["all", "follow"])]
    after: Option<i64>,
    /// Every line, following the pages to the end.
    #[arg(long, conflicts_with = "follow")]
    all: bool,
    /// Print the lines that arrive from now on, until the job is published, failed or
    /// cancelled. Earlier lines: run without --follow first.
    #[arg(long)]
    follow: bool,
}

impl LogArgs {
    /// Prints the lines, or with `--json` the K1 `BuildLogLine` list.
    ///
    /// # Errors
    /// [`Failure`] for an unknown job or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.follow {
            return follow::run(&self.job, invocation).await;
        }
        let hub = invocation.hub()?;
        let first = page_size(self.limit, self.all, MAX_PAGE);
        let (hub, job) = (&hub, &self.job);
        let log = fetch(self.after, self.all, move |after| async move {
            let variables = build_job_log::Variables {
                id: job.clone(),
                after,
                first: Some(first),
            };
            let lines = hub.run::<BuildJobLog>(variables).await?.build_job_log;
            let full = i64::try_from(lines.len()).is_ok_and(|count| count == first);
            Ok(LogPage { lines, full })
        })
        .await?;
        if invocation.output.json() {
            invocation
                .output
                .data(&log.lines, |_| crate::output::Table::record())?;
        } else {
            print_lines(&log.lines)?;
        }
        let command = CommandLine::new("anima release log", self.limit).word(&self.job);
        note_next(invocation, &log, "lines (maybe more)", &command);
        Ok(())
    }
}

/// Log lines and whether the page was full, which is the only sign there may be more: the
/// hub pages the log by line sequence, with no page info.
struct LogPage {
    lines: Lines,
    full: bool,
}

impl Page for LogPage {
    type Cursor = i64;

    fn next_cursor(&self) -> Option<i64> {
        self.lines
            .last()
            .map(|line| line.sequence)
            .filter(|_| self.full)
    }

    fn append(&mut self, later: Self) {
        self.lines.extend(later.lines);
        self.full = later.full;
    }

    fn end(&mut self) {
        self.full = false;
    }

    fn edge_count(&self) -> usize {
        self.lines.len()
    }
}

fn print_lines(lines: &Lines) -> Result<(), Failure> {
    let mut text = String::new();
    for line in lines {
        text.push_str(&clean(&line.text));
        text.push('\n');
    }
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => Err(Failure::Write {
            reason: error.to_string(),
        }),
        _ => Ok(()),
    }
}
