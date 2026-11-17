//! `anima daemon enrol`: a one-time code that turns on the metrics daemon of a device (MET-012).
//! Printing the code is the command's purpose; nothing else ever prints it.

use serde_json::json;

use super::documents::{DaemonEnrol, daemon_enrol};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::one_time_secret::{Secret, ToFile, show_once};

#[derive(Debug, clap::Args)]
pub struct EnrolArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl EnrolArgs {
    /// Prints the enrolment code and when it expires. The code works once; a new code makes
    /// the device's earlier unused codes useless.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = daemon_enrol::Variables {
            device_id: resolve(&hub, &self.device).await?,
        };
        let enrolment = hub.run::<DaemonEnrol>(variables).await?.enrol_daemon;
        invocation
            .output
            .note(&format!("The code expires {}.", enrolment.expires_at));
        let secret = Secret {
            label: "enrolment code",
            field: "code",
            value: &enrolment.code,
            to_file: &ToFile::default(),
        };
        show_once(
            invocation,
            &secret,
            json!({ "expiresAt": enrolment.expires_at }),
        )
    }
}
