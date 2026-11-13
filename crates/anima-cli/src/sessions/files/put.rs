//! `anima files put <local> <device>:<path>`: uploads one regular file. The hub never
//! replaces a file on the device (`EXISTS`).

use std::fs::File;
use std::path::PathBuf;

use super::describe;
use super::place::{Remote, upload_target};
use super::session::with_files;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct PutArgs {
    /// The local file.
    local: PathBuf,
    /// Where on the device, as `<device>:<path>`; a path ending in `/` keeps the local name.
    remote: String,
}

impl PutArgs {
    /// Uploads the file and notes where; `--json` prints `{"path", "size"}`.
    ///
    /// # Errors
    /// [`Failure::Input`] when the local file cannot be read; [`Failure::Conflict`] when
    /// the device path exists; the hub's refusal, a policy that forbids uploads, or a failed
    /// tunnel.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let remote = Remote::parse(&self.remote)?;
        let path = upload_target(&self.local, &remote)?;
        let unreadable = |reason: String| Failure::Input {
            what: self.local.display().to_string(),
            reason,
        };
        let mut file = File::open(&self.local).map_err(|error| unreadable(error.to_string()))?;
        let metadata = file
            .metadata()
            .map_err(|error| unreadable(error.to_string()))?;
        if !metadata.is_file() {
            return Err(unreadable("it is not a regular file".to_owned()));
        }
        let size = metadata.len();
        with_files(invocation, &remote.device, async |files| {
            files
                .upload(&path, &mut file, size)
                .await
                .map_err(|error| describe(&error, &path))
        })
        .await?;
        invocation
            .output
            .note(&format!("Uploaded {size} bytes to {path}."));
        if invocation.output.json() {
            let sent = serde_json::json!({ "path": path, "size": size });
            invocation.output.raw(&sent)?;
        }
        Ok(())
    }
}
