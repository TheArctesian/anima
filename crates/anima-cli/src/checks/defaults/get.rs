//! `anima check defaults get`: the values a new check starts with.

use super::documents::{CheckDefaultsGet, check_defaults_get};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct GetArgs;

impl GetArgs {
    /// Prints the defaults or, with `--json`, the K1 `CheckDefaults`.
    ///
    /// # Errors
    /// The hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<CheckDefaultsGet>(check_defaults_get::Variables)
            .await?;
        invocation.output.data(&answer.check_defaults, |defaults| {
            table(
                defaults.interval_seconds,
                defaults.retries,
                defaults.retry_interval_seconds,
                defaults.degraded_threshold_ms,
            )
        })
    }
}

/// The four defaults as label and value lines.
#[must_use]
pub fn table(interval: i64, retries: i64, retry_interval: i64, degraded: i64) -> Table {
    let mut table = Table::record();
    table.pair("Every", format!("{interval} s"));
    table.pair("Retries", retries.to_string());
    table.pair("Retry every", format!("{retry_interval} s"));
    table.pair("Degraded above", format!("{degraded} ms"));
    table
}
