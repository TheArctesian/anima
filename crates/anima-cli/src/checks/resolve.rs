//! A check given as an ID or as an exact name. A name is only unique on its device, so a name
//! needs `--device`.

use graphql_client::GraphQLQuery;

use crate::devices::get::resolve as resolve_device;
use crate::failure::Failure;
use crate::hub::Hub;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/checks/resolve.graphql",
    response_derives = "Debug",
    variables_derives = "Debug"
)]
pub struct CheckNames;

#[derive(Debug, clap::Args)]
pub struct CheckRef {
    /// The check's ID, or its exact name together with --device.
    check: String,
    /// The device that holds the check, by ID or exact name; needed when the check is a name.
    #[arg(long)]
    device: Option<String>,
}

impl CheckRef {
    /// What the operator typed, for questions and messages.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.check
    }

    /// The device the operator named, if any.
    #[must_use]
    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// The check's ID, as [`find`] returns it.
    ///
    /// # Errors
    /// As [`find`].
    pub async fn resolve(&self, hub: &Hub) -> Result<String, Failure> {
        find(hub, &self.check, self.device()).await
    }
}

/// The ID `check` names: an ID as it is, or the one check of `device` with exactly that name.
///
/// # Errors
/// [`Failure::Usage`] for a name without a device; [`Failure::NotFound`] when the device has
/// no check of that name; the hub's refusal.
pub async fn find(hub: &Hub, check: &str, device: Option<&str>) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(check).is_ok() {
        return Ok(check.to_owned());
    }
    let Some(device) = device else {
        return Err(Failure::Usage {
            message: format!(
                "{check} is not a check ID. Name its device too: --device <id-or-name>."
            ),
            hint: Some("anima check list --device <id-or-name>".to_owned()),
        });
    };
    let device_id = resolve_device(hub, device).await?;
    let answer = hub
        .run::<CheckNames>(check_names::Variables {
            device: device_id.clone(),
        })
        .await?;
    let checks = answer
        .device
        .ok_or_else(|| Failure::NotFound {
            message: format!("No device has the ID {device_id}."),
            hint: Some("anima device list".to_owned()),
        })?
        .checks;
    checks
        .into_iter()
        .find(|candidate| candidate.name == check)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("{device} has no check named {check}."),
            hint: Some(format!("anima check list --device {device}")),
        })
}
