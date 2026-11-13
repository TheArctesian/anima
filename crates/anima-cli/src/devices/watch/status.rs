//! `anima device watch`: one line for each status change of any device, or of one (K1
//! `deviceStatusChanged`).

use graphql_client::GraphQLQuery;

use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::scalars::DateTime;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::text::{clean, enum_name};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/watch/status.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct DeviceStatusChanged;

#[derive(Debug, clap::Args)]
pub struct WatchArgs {
    /// Only this device, by ID or exact name.
    #[arg(long)]
    device: Option<String>,
}

impl WatchArgs {
    /// Prints each change as a line (time, device ID, status) or, with `--json`, as one K1
    /// `DeviceStatusChange` object per line.
    ///
    /// # Errors
    /// [`Failure`] for an unknown device, or when the hub refuses the token or the connection
    /// is lost.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = match &self.device {
            Some(device) => Some(resolve(&hub, device).await?),
            None => None,
        };
        let variables = device_status_changed::Variables { device_id };
        let mut changes = Subscription::open::<DeviceStatusChanged>(&hub, variables).await?;
        while let Some(answer) = changes.next().await {
            let change = answer?.device_status_changed;
            if stream::emit(invocation.output.json(), &change, line)? == Reader::Gone {
                break;
            }
        }
        Ok(())
    }
}

fn line(change: &device_status_changed::DeviceStatusChangedDeviceStatusChanged) -> String {
    clean(&format!(
        "{}  {}  {}",
        change.at,
        change.device_id,
        enum_name(&change.status)
    ))
}
