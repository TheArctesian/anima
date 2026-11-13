//! `anima files get <device>:<path> [local]`: downloads one file. The bytes go to a temporary
//! file beside the target, which takes the target's name only when every byte came; an
//! existing file is replaced only with `--force`.

use std::io;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use super::describe;
use super::place::{Remote, download_target};
use super::session::with_files;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct GetArgs {
    /// The file, as `<device>:<path>` (the device by ID or exact name).
    remote: String,
    /// Where to save it: a file, or a folder to save it in. The current folder by default.
    local: Option<PathBuf>,
    /// Replace the local file when it exists.
    #[arg(long)]
    force: bool,
}

impl GetArgs {
    /// Saves the file and notes where; `--json` prints `{"path", "size"}`.
    ///
    /// # Errors
    /// [`Failure::Conflict`] when the local file exists without `--force`; the hub's
    /// refusal, a policy that forbids downloads, or a failed tunnel or write.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let remote = Remote::parse(&self.remote)?;
        let local_is_dir = self.local.as_deref().is_some_and(Path::is_dir);
        let target = download_target(&remote, self.local.as_deref(), local_is_dir)?;
        if !self.force && target.exists() {
            return Err(exists(&target));
        }
        let mut file = temporary_beside(&target)?;
        let path = remote.path.clone();
        let size = with_files(invocation, &remote.device, async |files| {
            files
                .download(&path, file.as_file_mut())
                .await
                .map_err(|error| describe(&error, &path))
        })
        .await?;
        let kept = if self.force {
            file.persist(&target).map_err(|error| error.error)
        } else {
            file.persist_noclobber(&target).map_err(|error| error.error)
        };
        kept.map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => exists(&target),
            _ => written(&target, &error),
        })?;
        let saved = target.display().to_string();
        invocation
            .output
            .note(&format!("Saved {saved} ({size} bytes)."));
        if invocation.output.json() {
            invocation
                .output
                .raw(&serde_json::json!({ "path": saved, "size": size }))?;
        }
        Ok(())
    }
}

/// An empty temporary file in the target's folder, so the final rename stays on one disk.
fn temporary_beside(target: &Path) -> Result<NamedTempFile, Failure> {
    let folder = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    tempfile::Builder::new()
        .prefix(".anima-download-")
        .tempfile_in(folder)
        .map_err(|error| written(target, &error))
}

fn exists(target: &Path) -> Failure {
    Failure::Conflict {
        message: format!(
            "{} already exists; pass --force to replace it.",
            target.display()
        ),
    }
}

fn written(target: &Path, error: &io::Error) -> Failure {
    Failure::Write {
        reason: format!("{}: {error}", target.display()),
    }
}
