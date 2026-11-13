//! `anima vpn invite` and `anima vpn router-key`: a one-use, one-hour pre-auth key (VPN-001,
//! VPN-003). The key is the one secret these commands exist to show: on standard output, once,
//! or with `--to-file` in a `0600` file and nowhere else. It never goes on a command line, into
//! a log or into a note (CMD-014, LOG-007).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::approve_route::resolve_site;
use super::documents::{VpnInvite, VpnRouterKey, vpn_invite, vpn_router_key};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::private_file;

#[derive(Debug, clap::Args)]
pub struct InviteArgs {
    /// Write the key to this file (mode 0600) instead of standard output.
    #[arg(long, value_name = "PATH")]
    to_file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct RouterKeyArgs {
    /// The site the subnet router serves, by name or ID.
    #[arg(long)]
    site: String,
    /// Write the key to this file (mode 0600) instead of standard output.
    #[arg(long, value_name = "PATH")]
    to_file: Option<PathBuf>,
}

/// A secret a command shows once, and where it goes.
pub struct Secret<'a> {
    /// What the person calls it: "pre-auth key".
    pub label: &'a str,
    /// Its field in the `--json` answer.
    pub field: &'a str,
    pub value: &'a str,
    pub to_file: Option<&'a Path>,
}

impl InviteArgs {
    /// Makes the key for a Linux computer of the operator and shows it once.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (the broker's limits) or a file that cannot be written.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<VpnInvite>(vpn_invite::Variables)
            .await?
            .create_vpn_invite;
        let key = Zeroizing::new(answer.key);
        invocation.output.note(&steps(&answer.steps));
        let public = json!({ "expiresAt": answer.expires_at, "steps": answer.steps });
        let secret = Secret {
            label: "pre-auth key",
            field: "key",
            value: &key,
            to_file: self.to_file.as_deref(),
        };
        show_once(invocation, &secret, public)
    }
}

impl RouterKeyArgs {
    /// Makes the key for the subnet router of a site and shows it once.
    ///
    /// # Errors
    /// [`Failure`] for an unknown site, a hub refusal or a file that cannot be written.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let site_id = resolve_site(&hub, &self.site).await?;
        let answer = hub
            .run::<VpnRouterKey>(vpn_router_key::Variables { site_id })
            .await?
            .create_router_key;
        let key = Zeroizing::new(answer.key);
        invocation.output.note(&steps(&answer.steps));
        let public = json!({ "expiresAt": answer.expires_at, "steps": answer.steps });
        let secret = Secret {
            label: "router key",
            field: "key",
            value: &key,
            to_file: self.to_file.as_deref(),
        };
        show_once(invocation, &secret, public)
    }
}

/// Shows `secret` once: as a line on standard output, or in its file with nothing secret
/// printed. With `--json`, `public` is the answer, with the secret under its field unless it
/// went to a file. The notes go to standard error.
///
/// # Errors
/// [`Failure::Write`] when the file or standard output cannot be written.
pub fn show_once(
    invocation: &Invocation,
    secret: &Secret<'_>,
    mut public: Value,
) -> Result<(), Failure> {
    match secret.to_file {
        Some(path) => {
            write_file(path, secret)?;
            invocation.output.note(&format!(
                "The {} is in {} (mode 0600). It is shown one time and works once.",
                secret.label,
                path.display()
            ));
            if invocation.output.json() {
                public["writtenTo"] = json!(path.display().to_string());
                invocation.output.raw(&public)?;
            }
            Ok(())
        }
        None => {
            invocation.output.note(&format!(
                "The {} below is shown one time and works once. Keep it out of command lines and logs.",
                secret.label
            ));
            if invocation.output.json() {
                public[secret.field] = json!(secret.value);
                return invocation.output.raw(&public);
            }
            write_line(secret.value)
        }
    }
}

fn steps(steps: &[String]) -> String {
    let mut text = String::from("Steps for the new computer, in order:");
    for (number, step) in steps.iter().enumerate() {
        text.push_str(&format!("\n  {}. {step}", number + 1));
    }
    text
}

fn write_file(path: &Path, secret: &Secret<'_>) -> Result<(), Failure> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Failure::Usage {
            message: "--to-file needs a path that ends in a file name.".to_owned(),
            hint: None,
        })?;
    private_file::write(dir, name, secret.value.as_bytes()).map_err(|error| Failure::Write {
        reason: format!(
            "{}: {error}. The {} is lost; make another",
            path.display(),
            secret.label
        ),
    })
}

fn write_line(text: &str) -> Result<(), Failure> {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(format!("{text}\n").as_bytes())
        .and_then(|()| stdout.flush())
    {
        Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => Err(Failure::Write {
            reason: error.to_string(),
        }),
        _ => Ok(()),
    }
}
