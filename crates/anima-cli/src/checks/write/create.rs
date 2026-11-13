//! `anima check create`: a check on a device. Its kind and settings come from a JSON file
//! (`--from-file`, `-` for standard input) holding the K1 `CheckConfigInput`, such as
//! `{"tcp": {"port": 22}}`.

use std::path::PathBuf;

use super::documents::{CheckCreate, check_create};
use crate::checks::config_file;
use crate::checks::resolve::find;
use crate::checks::summary::{state, written};
use crate::devices::get::resolve as resolve_device;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The device to check, by ID or exact name.
    #[arg(long)]
    device: String,
    /// The check's name.
    #[arg(long)]
    name: String,
    /// A JSON file with the kind and settings (K1 `CheckConfigInput`), or - for standard input.
    #[arg(long, value_name = "FILE")]
    from_file: PathBuf,
    /// A group check on the same device that holds this one, by ID or exact name.
    #[arg(long)]
    parent: Option<String>,
    /// Seconds between runs (default 60).
    #[arg(long)]
    interval_seconds: Option<i64>,
    /// Failed runs in a row before the check is Down (default 3).
    #[arg(long)]
    retries: Option<i64>,
    /// Seconds between retries (default 20).
    #[arg(long)]
    retry_interval_seconds: Option<i64>,
    /// Response time above which the check is Degraded, in milliseconds (default 1000).
    #[arg(long)]
    degraded_threshold_ms: Option<i64>,
    /// Count a failure as Up and a success as Down.
    #[arg(long)]
    invert: bool,
}

impl CreateArgs {
    /// Prints the new check.
    ///
    /// # Errors
    /// [`Failure::Usage`] for a bad configuration file or an unknown name; the hub's refusal
    /// (VALIDATION for settings the hub rejects).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let config = config_file::read(&self.from_file)?;
        let hub = invocation.hub()?;
        let device_id = resolve_device(&hub, &self.device).await?;
        let parent_id = match &self.parent {
            Some(parent) => Some(find(&hub, parent, Some(&self.device)).await?),
            None => None,
        };
        let input = check_create::CreateCheckInput {
            device_id,
            parent_id,
            name: self.name,
            config,
            interval_seconds: self.interval_seconds,
            retries: self.retries,
            retry_interval_seconds: self.retry_interval_seconds,
            degraded_threshold_ms: self.degraded_threshold_ms,
            invert: self.invert.then_some(true),
        };
        let answer = hub
            .run::<CheckCreate>(check_create::Variables { input })
            .await?;
        invocation.output.data(&answer.create_check, |check| {
            let status = state(&enum_name(&check.status), check.paused);
            written(&check.name, &check.id, &enum_name(&check.type_), &status)
        })?;
        invocation
            .output
            .note("A push check's URL is in: anima check get <check>");
        Ok(())
    }
}
