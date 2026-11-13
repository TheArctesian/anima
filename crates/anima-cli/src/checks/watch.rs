//! `anima check watch`: one line for each heartbeat of the checks of the hub, of one device, or
//! of one check (K1 `heartbeatAdded`).

use graphql_client::GraphQLQuery;

use super::resolve::find;
use crate::devices::get::resolve as resolve_device;
use crate::failure::Failure;
use crate::hub::scalars::DateTime;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::text::{clean, enum_name};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug",
    skip_serializing_none
)]
pub struct HeartbeatAdded;

#[derive(Debug, clap::Args)]
pub struct WatchArgs {
    /// Only this check: its ID, or its exact name together with --device.
    #[arg(long)]
    check: Option<String>,
    /// Only this device's checks, by ID or exact name.
    #[arg(long)]
    device: Option<String>,
}

impl WatchArgs {
    /// Prints each heartbeat as a line (time, check ID, status, response time, message) or,
    /// with `--json`, as one K1 `HeartbeatAdded` object per line.
    ///
    /// # Errors
    /// [`Failure`] for an unknown check or device, or when the hub refuses the token or the
    /// connection is lost.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let check_id = match &self.check {
            Some(check) => Some(find(&hub, check, self.device.as_deref()).await?),
            None => None,
        };
        let device_id = match &self.device {
            Some(device) => Some(resolve_device(&hub, device).await?),
            None => None,
        };
        let variables = heartbeat_added::Variables {
            check_id,
            device_id,
        };
        let mut beats = Subscription::open::<HeartbeatAdded>(&hub, variables).await?;
        while let Some(answer) = beats.next().await {
            let added = answer?.heartbeat_added;
            if stream::emit(invocation.output.json(), &added, line)? == Reader::Gone {
                break;
            }
        }
        Ok(())
    }
}

fn line(added: &heartbeat_added::HeartbeatAddedHeartbeatAdded) -> String {
    let beat = &added.heartbeat;
    let response = beat
        .response_time_ms
        .map_or_else(|| "-".to_owned(), |ms| format!("{ms} ms"));
    clean(&format!(
        "{}  {}  {}  {response}  {}",
        beat.at,
        added.check_id,
        enum_name(&beat.status),
        beat.message.as_deref().unwrap_or("")
    ))
}
