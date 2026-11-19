//! `anima file upload <path>` and `anima file download <id> --output <path>`: the bytes stream
//! from a file of this computer and into a new one, never through a terminal (FIL-005).

use std::path::{Path, PathBuf};

use tokio::fs::{File, OpenOptions};

use crate::failure::Failure;
use crate::hub::attachments::Upload;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct UploadArgs {
    /// The file of this computer to send.
    path: PathBuf,
    /// The record the file belongs to, as kind:id.
    #[arg(long)]
    owner: String,
    /// The name to keep; the name of the file when not given.
    #[arg(long)]
    name: Option<String>,
    /// The data key that seals it: `engine` or the key of the extension that owns the record.
    #[arg(long, default_value = "engine")]
    key: String,
    /// The archive of the import it belongs to (see `anima file archive add`).
    #[arg(long)]
    archive: Option<String>,
    /// Keep the GPS position of a photo as a field of the file (FIL-003). Without it the hub
    /// keeps no position.
    #[arg(long)]
    keep_location: bool,
    /// The media type; from the name of the file when not given.
    #[arg(long = "type")]
    media_type: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct DownloadArgs {
    /// The file's ID (see `anima file list`).
    id: String,
    /// The new file of this computer to write. A file that exists is not replaced.
    #[arg(long, short)]
    output: PathBuf,
}

/// The media type of a file from its extension; `application/octet-stream` for any other.
#[must_use]
pub fn media_type_of(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("heic") => "image/heic",
        Some("pdf") => "application/pdf",
        Some("json") => "application/json",
        Some("txt") => "text/plain",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
}

fn local(what: &Path, error: &std::io::Error) -> Failure {
    Failure::Input {
        what: what.display().to_string(),
        reason: error.kind().to_string(),
    }
}

impl UploadArgs {
    /// Sends the file and prints the attachment.
    ///
    /// # Errors
    /// [`Failure::Input`] when the file cannot be read; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let file = File::open(&self.path)
            .await
            .map_err(|error| local(&self.path, &error))?;
        let length = file
            .metadata()
            .await
            .map_err(|error| local(&self.path, &error))?
            .len();
        let name = match &self.name {
            Some(name) => name.clone(),
            None => (self.path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .ok_or_else(|| Failure::missing("the name of the file", "--name"))?,
        };
        let media_type = (self.media_type.as_deref()).unwrap_or_else(|| media_type_of(&self.path));
        let upload = Upload {
            owner: &self.owner,
            name: &name,
            key: &self.key,
            archive: self.archive.as_deref(),
            keep_location: self.keep_location,
        };
        let answer = hub
            .upload_attachment(&upload, (file, length), media_type)
            .await?;
        invocation.output.data(&answer, |attachment| {
            let mut table = Table::record();
            for (label, key) in [
                ("ID", "id"),
                ("Name", "name"),
                ("Type", "mediaType"),
                ("Hash", "hash"),
            ] {
                table.pair(label, attachment[key].as_str().unwrap_or("-"));
            }
            table.pair("Size", attachment["size"].to_string().as_str());
            table
        })
    }
}

impl DownloadArgs {
    /// Writes the bytes to a new file, and says whether the hub removed the location of a photo.
    /// A download that fails leaves no file.
    ///
    /// # Errors
    /// [`Failure::Input`] when the file exists or cannot be made; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.output)
            .await
            .map_err(|error| local(&self.output, &error))?;
        let written = hub.download_attachment(&self.id, &mut file).await;
        let downloaded = match written {
            Ok(downloaded) => downloaded,
            Err(error) => {
                drop(file);
                // The refusal is the failure to report; a partial file that is gone already is
                // not a second one.
                let _ = tokio::fs::remove_file(&self.output).await;
                return Err(error.into());
            }
        };
        invocation.output.data(&downloaded, |downloaded| {
            let mut table = Table::record();
            table.pair("Written", self.output.display().to_string().as_str());
            table.pair("Size", downloaded.size.to_string().as_str());
            let removed = if downloaded.location_removed {
                "yes"
            } else {
                "no"
            };
            table.pair("Location removed", removed);
            table
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::media_type_of;

    #[test]
    fn fil_003_a_photo_is_sent_as_a_jpeg_by_its_name() {
        assert_eq!(media_type_of(Path::new("IMG_1.JPG")), "image/jpeg");
        assert_eq!(
            media_type_of(Path::new("notes")),
            "application/octet-stream"
        );
    }
}
