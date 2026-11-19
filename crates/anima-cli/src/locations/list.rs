//! `anima location list [--from <time>] [--to <time>] [--first <n>]`. The hub writes one audit
//! entry for each read.

use chrono::Utc;

use super::documents::{LocationList, location_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;
use crate::timeline::window::window;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Start of the span (RFC 3339, such as 2026-11-01T00:00:00Z); one day before --to when
    /// not given.
    #[arg(long)]
    from: Option<String>,
    /// End of the span (RFC 3339); now when not given. A span is at most 31 days.
    #[arg(long)]
    to: Option<String>,
    /// The most samples, 1 to 1000.
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(i64).range(1..=1000))]
    first: i64,
}

impl ListArgs {
    /// Prints the samples as a table or, with `--json`, as the K1 `LocationSample` list.
    ///
    /// # Errors
    /// [`Failure::Usage`] for an unusable `--from` or `--to`; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let (from, to) =
            window(self.from.as_deref(), self.to.as_deref(), Utc::now()).map_err(|error| {
                Failure::Usage {
                    message: error.to_string(),
                    hint: None,
                }
            })?;
        let hub = invocation.hub()?;
        let variables = location_list::Variables {
            from,
            to,
            first: Some(self.first),
        };
        let answer = hub.run::<LocationList>(variables).await?;
        invocation.output.data(&answer.location_samples, table)
    }
}

/// A number with `places` decimals, or `?` when the provider gave none: unknown, not zero.
#[must_use]
pub fn number(value: Option<f64>, places: usize) -> String {
    value.map_or_else(|| "?".to_owned(), |value| format!("{value:.places$}"))
}

fn table(samples: &Vec<location_list::LocationListLocationSamples>) -> Table {
    let mut table = Table::new(vec![
        "OBSERVED",
        "PROVIDER",
        "LATITUDE",
        "LONGITUDE",
        "ACCURACY M",
        "RECEIVED",
        "ID",
    ]);
    for sample in samples {
        table.row(vec![
            sample.observed_at.as_deref().unwrap_or("?").into(),
            enum_name(&sample.provider).as_str().into(),
            number(sample.latitude, 5).as_str().into(),
            number(sample.longitude, 5).as_str().into(),
            number(sample.accuracy_m, 0).as_str().into(),
            sample.received_at.as_str().into(),
            sample.id.as_str().into(),
        ]);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn r_145_a_missing_accuracy_shows_it_is_unknown() {
        assert_eq!(number(Some(12.4), 0), "12");
        assert_eq!(number(None, 0), "?");
    }
}
