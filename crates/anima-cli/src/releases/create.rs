//! `anima release create`: starts a release with one build job for each target (BLD-001).

use super::documents::{ReleaseCreate, release_create};
use super::list::job_summary;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The version to publish, such as 0.4.0.
    #[arg(long)]
    version: String,
    /// The Git commit to build, 40 lower-case hex digits.
    #[arg(long)]
    commit: String,
    /// What to build for; repeat for more than one.
    #[arg(long, required = true, value_enum)]
    target: Vec<TargetArg>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum TargetArg {
    #[value(name = "linux-x86-64")]
    LinuxX8664,
    DarwinUniversal,
}

impl CreateArgs {
    /// Prints the new release or, with `--json`, the K1 `Release`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (a version that exists, a commit that is not there).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = release_create::Variables {
            version: self.version,
            commit: self.commit,
            targets: self.target.iter().map(|target| target.k1()).collect(),
        };
        let answer = hub.run::<ReleaseCreate>(variables).await?;
        invocation.output.data(&answer.create_release, table)
    }
}

fn table(release: &release_create::ReleaseCreateCreateRelease) -> Table {
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
    let mut table = Table::record();
    table.pair("Version", release.version.as_str());
    table.pair("Commit", release.commit.as_str());
    table.pair("ID", release.id.as_str());
    table.pair("Created", release.created_at.as_str());
    table.pair("By", release.created_by.as_str());
    table.pair("Jobs", jobs);
    table
}

impl TargetArg {
    const fn k1(self) -> release_create::BuildTarget {
        match self {
            Self::LinuxX8664 => release_create::BuildTarget::LINUX_X86_64,
            Self::DarwinUniversal => release_create::BuildTarget::DARWIN_UNIVERSAL,
        }
    }
}
