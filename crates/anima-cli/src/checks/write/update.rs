//! `anima check update <id-or-name>`: changes the settings you name and leaves the rest.
//! `--from-file` replaces the kind's settings as a whole (K1 `CheckConfigInput`).

use std::path::PathBuf;

use super::documents::{CheckUpdate, CheckUpdateUngrouped, check_update, check_update_ungrouped};
use crate::checks::config_file;
use crate::checks::resolve::{CheckRef, find};
use crate::checks::summary::{state, written};
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    #[command(flatten)]
    check: CheckRef,
    /// A new name.
    #[arg(long)]
    name: Option<String>,
    /// Replace the settings with this JSON file (K1 `CheckConfigInput`), or - for standard input.
    #[arg(long, value_name = "FILE")]
    from_file: Option<PathBuf>,
    /// Move the check into this group check, by ID or exact name.
    #[arg(long)]
    parent: Option<String>,
    /// Take the check out of its group.
    #[arg(long, conflicts_with = "parent")]
    no_parent: bool,
    /// Seconds between runs.
    #[arg(long)]
    interval_seconds: Option<i64>,
    /// Failed runs in a row before the check is Down.
    #[arg(long)]
    retries: Option<i64>,
    /// Seconds between retries.
    #[arg(long)]
    retry_interval_seconds: Option<i64>,
    /// Response time above which the check is Degraded, in milliseconds.
    #[arg(long)]
    degraded_threshold_ms: Option<i64>,
    /// Count a failure as Up and a success as Down: true or false.
    #[arg(long)]
    invert: Option<bool>,
}

impl UpdateArgs {
    /// Prints the changed check.
    ///
    /// # Errors
    /// [`Failure::Usage`] when nothing is to change or the file is wrong; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let unchanged = self.from_file.is_none()
            && self.name.is_none()
            && self.parent.is_none()
            && !self.no_parent
            && self.interval_seconds.is_none()
            && self.retries.is_none()
            && self.retry_interval_seconds.is_none()
            && self.degraded_threshold_ms.is_none()
            && self.invert.is_none();
        if unchanged {
            return Err(Failure::Usage {
                message: "Nothing to change: give at least one setting to update.".to_owned(),
                hint: Some(Hint::run("anima check update --help")),
            });
        }
        if self.no_parent {
            return self.run_ungrouped(invocation).await;
        }
        let config = self
            .from_file
            .as_deref()
            .map(|path| config_file::read(invocation.prompter, path))
            .transpose()?;
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let parent_id = match &self.parent {
            Some(parent) => Some(find(&hub, parent, self.check.device()).await?),
            None => None,
        };
        let input = check_update::UpdateCheckInput {
            parent_id,
            name: self.name,
            config,
            interval_seconds: self.interval_seconds,
            retries: self.retries,
            retry_interval_seconds: self.retry_interval_seconds,
            degraded_threshold_ms: self.degraded_threshold_ms,
            invert: self.invert,
        };
        let answer = hub
            .run::<CheckUpdate>(check_update::Variables { id, input })
            .await?;
        invocation.output.data(&answer.update_check, |check| {
            let status = state(&enum_name(&check.status), check.paused);
            written(&check.name, &check.id, &enum_name(&check.type_), &status)
        })
    }

    /// The change with an explicit null parent, which the generated input cannot send.
    async fn run_ungrouped(self, invocation: &Invocation) -> Result<(), Failure> {
        let config = self
            .from_file
            .as_deref()
            .map(|path| config_file::read(invocation.prompter, path))
            .transpose()?;
        let hub = invocation.hub()?;
        let id = self.check.resolve(&hub).await?;
        let variables = check_update_ungrouped::Variables {
            id,
            name: self.name,
            config,
            interval_seconds: self.interval_seconds,
            retries: self.retries,
            retry_interval_seconds: self.retry_interval_seconds,
            degraded_threshold_ms: self.degraded_threshold_ms,
            invert: self.invert,
        };
        let answer = hub.run::<CheckUpdateUngrouped>(variables).await?;
        invocation.output.data(&answer.update_check, |check| {
            let status = state(&enum_name(&check.status), check.paused);
            written(&check.name, &check.id, &enum_name(&check.type_), &status)
        })
    }
}
