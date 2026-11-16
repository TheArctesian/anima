//! `anima link invite <name>`: a single-use invite for a new device (LNK-003). The invite is
//! the one secret this command exists to show: on standard output, once, or with `--to-file`
//! in a `0600` file and nowhere else. It never goes on a command line, into a log or into a
//! note (CMD-014, LOG-007).

use serde_json::json;
use zeroize::Zeroizing;

use super::documents::{LinkInvite, link_invite};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::one_time_secret::{Secret, ToFile, show_once};

#[derive(Debug, clap::Args)]
pub struct InviteArgs {
    /// The new device's name, 1 to 64 visible characters.
    name: String,
    #[command(flatten)]
    to_file: ToFile,
}

impl InviteArgs {
    /// Makes the invite and shows it once; its expiry goes to standard error (or the `--json`
    /// answer).
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (`VALIDATION` for the name, `FORBIDDEN` for an API key,
    /// `UNAVAILABLE` when the hub has no link) or a file that cannot be written.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        self.to_file.check()?;
        let hub = invocation.hub()?;
        let answer = hub
            .run::<LinkInvite>(link_invite::Variables {
                name: self.name.clone(),
            })
            .await?
            .create_link_invite;
        let invite = Zeroizing::new(answer.invite);
        invocation.output.note(&format!(
            "Invite for the device {}; it expires {}. On the device: anima-link join, then paste it.",
            self.name, answer.expires_at
        ));
        let public = json!({ "name": self.name, "expiresAt": answer.expires_at });
        let secret = Secret {
            label: "invite",
            field: "invite",
            value: &invite,
            to_file: &self.to_file,
        };
        show_once(invocation, &secret, public)
    }
}
