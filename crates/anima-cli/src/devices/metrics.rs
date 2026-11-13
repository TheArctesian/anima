//! `anima device metrics`: the metric series of a device over an hour, a day or a week, or with
//! `--watch` each new reading.

use super::documents::{DeviceMetrics, device_metrics};
use super::get::resolve;
use super::watch;
use crate::failure::{Failure, Hint};
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

#[derive(Debug, clap::Args)]
pub struct MetricsArgs {
    /// The device's ID or its exact name.
    device: String,
    /// How far back.
    #[arg(long, value_enum, default_value_t = RangeArg::Hour)]
    range: RangeArg,
    /// Print each new reading until interrupted, instead of the series.
    #[arg(long, conflicts_with = "range")]
    watch: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum RangeArg {
    Hour,
    Day,
    Week,
}

impl MetricsArgs {
    /// Prints one row for each series, or with `--json` every point.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.watch {
            return watch::metrics::run(&self.device, invocation).await;
        }
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let variables = device_metrics::Variables {
            id: id.clone(),
            range: self.range.k1(),
        };
        let device = hub
            .run::<DeviceMetrics>(variables)
            .await?
            .device
            .ok_or_else(|| Failure::NotFound {
                message: format!("No device has the ID {id}."),
                hint: Some(Hint::run("anima device list")),
            })?;
        invocation.output.data(&device, table)
    }
}

fn table(device: &device_metrics::DeviceMetricsDevice) -> Table {
    let mut table = Table::new(vec!["KIND", "LABEL", "POINTS", "LATEST", "MIN", "MAX"]);
    for series in &device.metrics {
        let values = series.points.iter().map(|point| point.value);
        let latest = series.points.last().map(|point| point.value);
        table.row(vec![
            enum_name(&series.kind).into(),
            series.label.as_str().into(),
            series.points.len().to_string().into(),
            number(latest),
            number(values.clone().reduce(f64::min)),
            number(values.reduce(f64::max)),
        ]);
    }
    table
}

fn number(value: Option<f64>) -> Cell {
    value.map_or_else(|| "-".into(), |value| format!("{value:.2}").into())
}

impl RangeArg {
    const fn k1(self) -> device_metrics::MetricRange {
        match self {
            Self::Hour => device_metrics::MetricRange::HOUR,
            Self::Day => device_metrics::MetricRange::DAY,
            Self::Week => device_metrics::MetricRange::WEEK,
        }
    }
}
