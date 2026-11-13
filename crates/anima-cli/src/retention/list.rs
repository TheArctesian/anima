//! `anima retention list`: the days kept for each kind of record.

use super::documents::{RetentionList, retention_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct ListArgs;

impl ListArgs {
    /// Prints the periods as a table or, with `--json`, as the K1 `RetentionPeriod` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<RetentionList>(retention_list::Variables).await?;
        invocation.output.data(&answer.retention, table)
    }
}

fn table(periods: &Vec<retention_list::RetentionListRetention>) -> Table {
    let mut table = Table::new(vec!["SUBJECT", "DAYS", "MIN", "MAX", "DEFAULT"]);
    for period in periods {
        table.row(vec![
            enum_name(&period.subject).into(),
            period.days.to_string().into(),
            period.min_days.to_string().into(),
            period.max_days.to_string().into(),
            period.default_days.to_string().into(),
        ]);
    }
    table
}
