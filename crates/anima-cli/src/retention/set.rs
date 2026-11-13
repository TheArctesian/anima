//! `anima retention set <subject> <days>`.

use super::documents::{RetentionSet, retention_set};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// The kind of record.
    #[arg(value_enum)]
    subject: SubjectArg,
    /// How many days to keep it; the hub names the allowed range when it is outside.
    days: i64,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum SubjectArg {
    Heartbeats,
    Metrics,
    Events,
    Deliveries,
}

impl SetArgs {
    /// Prints the new period as a record or, with `--json`, as the K1 `RetentionPeriod`.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (`VALIDATION` for days outside the range, `FORBIDDEN`
    /// for an API key).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = retention_set::Variables {
            subject: self.subject.k1(),
            days: self.days,
        };
        let answer = hub.run::<RetentionSet>(variables).await?;
        invocation.output.data(&answer.set_retention, table)
    }
}

fn table(period: &retention_set::RetentionSetSetRetention) -> Table {
    let mut table = Table::record();
    table.pair("Subject", enum_name(&period.subject));
    table.pair("Days", period.days.to_string());
    table.pair(
        "Range",
        format!(
            "{} to {} (default {})",
            period.min_days, period.max_days, period.default_days
        ),
    );
    table
}

impl SubjectArg {
    const fn k1(self) -> retention_set::RetentionSubject {
        match self {
            Self::Heartbeats => retention_set::RetentionSubject::HEARTBEATS,
            Self::Metrics => retention_set::RetentionSubject::METRICS,
            Self::Events => retention_set::RetentionSubject::EVENTS,
            Self::Deliveries => retention_set::RetentionSubject::DELIVERIES,
        }
    }
}
