//! `anima release log <job id>`: the stored log of a build job, oldest line first (BLD-048).

use std::io::Write as _;

use super::documents::{BuildJobLog, build_job_log};
use super::follow;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::clean;

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
        let first = if self.all { MAX_PAGE } else { self.limit };
        let request = Request {
            job: &self.job,
            first,
            all: self.all,
        };
        let lines = fetch(&hub, &request, self.after).await?;
        if invocation.output.json() {
            invocation
                .output
                .data(&lines, |_| crate::output::Table::record())?;
        } else {
            print_lines(&lines)?;
        }
        let full = i64::try_from(lines.len()).is_ok_and(|count| count == first);
        if let Some(last) = lines.last().filter(|_| full && !self.all) {
            invocation.output.note(&format!(
                "Maybe more. Next: anima release log {} --after {} (or --all)",
                self.job, last.sequence
            ));
        }
        Ok(())
    }
}

/// What to ask for: the job, the page size, and whether to follow every page.
struct Request<'a> {
    job: &'a str,
    first: i64,
    all: bool,
}

/// One page after `after`, or with `all` pages until one is short.
async fn fetch(hub: &Hub, request: &Request<'_>, after: Option<i64>) -> Result<Lines, Failure> {
    let mut lines: Lines = Vec::new();
    let mut after = after;
    loop {
        let variables = build_job_log::Variables {
            id: request.job.to_owned(),
            after,
            first: Some(request.first),
        };
        let page = hub.run::<BuildJobLog>(variables).await?.build_job_log;
        let full = i64::try_from(page.len()).is_ok_and(|count| count == request.first);
        after = page.last().map(|line| line.sequence).or(after);
        lines.extend(page);
        if !(request.all && full) {
            return Ok(lines);
        }
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
