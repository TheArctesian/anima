//! `anima device metrics <device> --watch`: one line for each new reading of a device (K1
//! `metricsAdded`).

use graphql_client::GraphQLQuery;

use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::scalars::DateTime;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/watch/metrics.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct MetricsAdded;

/// Prints each reading of `device` (an ID or exact name) as a line (time, CPU, memory, disk)
/// or, with `--json`, as one K1 `MetricsAdded` object per line.
///
/// # Errors
/// [`Failure`] for an unknown device, or when the hub refuses the token or the connection is
/// lost.
pub async fn run(device: &str, invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let device_id = resolve(&hub, device).await?;
    let variables = metrics_added::Variables {
        device_id: Some(device_id),
    };
    let mut readings = Subscription::open::<MetricsAdded>(&hub, variables).await?;
    while let Some(answer) = readings.next().await {
        let added = answer?.metrics_added;
        if stream::emit(invocation.output.json(), &added, line)? == Reader::Gone {
            break;
        }
    }
    Ok(())
}

fn line(added: &metrics_added::MetricsAddedMetricsAdded) -> String {
    let readings = &added.readings;
    format!(
        "{}  cpu {}  memory {}  disk {}",
        added.at,
        percent(readings.cpu_percent),
        percent(readings.memory_percent),
        percent(readings.disk_percent)
    )
}

fn percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}%"))
}
