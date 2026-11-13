//! `anima check defaults set`: changes the values you name and keeps the others.

use super::documents::{
    CheckDefaultsGet, CheckDefaultsSet, check_defaults_get, check_defaults_set,
};
use super::get::table;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// Seconds between runs.
    #[arg(long)]
    interval_seconds: Option<i64>,
    /// Failed runs in a row before a check is Down.
    #[arg(long)]
    retries: Option<i64>,
    /// Seconds between retries.
    #[arg(long)]
    retry_interval_seconds: Option<i64>,
    /// Response time above which a check is Degraded, in milliseconds.
    #[arg(long)]
    degraded_threshold_ms: Option<i64>,
}

impl SetArgs {
    /// Prints the defaults as they are now.
    ///
    /// # Errors
    /// [`Failure::Usage`] when no value is given; the hub's refusal (VALIDATION for a value
    /// out of bounds).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.interval_seconds.is_none()
            && self.retries.is_none()
            && self.retry_interval_seconds.is_none()
            && self.degraded_threshold_ms.is_none()
        {
            return Err(Failure::Usage {
                message: "Nothing to change: give at least one value to set.".to_owned(),
                hint: Some("anima check defaults set --help".to_owned()),
            });
        }
        let hub = invocation.hub()?;
        let current = hub
            .run::<CheckDefaultsGet>(check_defaults_get::Variables)
            .await?
            .check_defaults;
        let input = check_defaults_set::CheckDefaultsInput {
            interval_seconds: self.interval_seconds.unwrap_or(current.interval_seconds),
            retries: self.retries.unwrap_or(current.retries),
            retry_interval_seconds: self
                .retry_interval_seconds
                .unwrap_or(current.retry_interval_seconds),
            degraded_threshold_ms: self
                .degraded_threshold_ms
                .unwrap_or(current.degraded_threshold_ms),
        };
        let answer = hub
            .run::<CheckDefaultsSet>(check_defaults_set::Variables { input })
            .await?;
        invocation
            .output
            .data(&answer.set_check_defaults, |defaults| {
                table(
                    defaults.interval_seconds,
                    defaults.retries,
                    defaults.retry_interval_seconds,
                    defaults.degraded_threshold_ms,
                )
            })
    }
}
