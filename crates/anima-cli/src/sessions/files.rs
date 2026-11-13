//! `anima files`: list, download and upload over the files tunnel (K4 Section 9, RA-032).
//! Login token only (INT-003). Each command opens an SSH session for the tunnel and ends it.

pub mod codec;
pub mod exchange;
pub mod get;
pub mod ls;
pub mod place;
pub mod put;
pub mod session;

use exchange::FilesError;

use crate::failure::Failure;
use crate::hub::{Code, HubError};
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum FilesCommand {
    /// List a folder on a device: `anima files ls web-1:/var/log`.
    Ls(ls::LsArgs),
    /// Download a file: `anima files get web-1:/etc/hosts [local]`.
    Get(get::GetArgs),
    /// Upload a file; never replaces one: `anima files put ./a.txt web-1:/tmp/`.
    Put(put::PutArgs),
}

impl FilesCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, a policy that forbids the transfer, or a failed tunnel.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Ls(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Put(args) => args.run(invocation).await,
        }
    }
}

/// `error` for the operator, about the device path `path`.
#[must_use]
pub fn describe(error: &FilesError, path: &str) -> Failure {
    let refused = |code: Code, message: String| {
        Failure::Hub(HubError::Refused {
            code,
            message,
            fields: Vec::new(),
        })
    };
    match error {
        FilesError::Refused(codec::Refusal::DownloadOff) => refused(
            Code::Forbidden,
            "The device's transfer policy does not allow downloads.".to_owned(),
        ),
        FilesError::Refused(codec::Refusal::UploadOff) => refused(
            Code::Forbidden,
            "The device's transfer policy does not allow uploads.".to_owned(),
        ),
        FilesError::Refused(codec::Refusal::Denied) => refused(
            Code::Forbidden,
            format!("The device refused access to {path}."),
        ),
        FilesError::Refused(codec::Refusal::NotFound) => Failure::NotFound {
            message: format!("There is no such file or folder on the device: {path}."),
            hint: None,
        },
        FilesError::Refused(codec::Refusal::Exists) => Failure::Conflict {
            message: format!(
                "{path} already exists on the device; an upload never replaces a file."
            ),
        },
        FilesError::Unsendable => Failure::Usage {
            message: "The device path is empty, holds NUL or is longer than 4096 bytes.".to_owned(),
            hint: None,
        },
        FilesError::Protocol => refused(Code::Internal, error.to_string()),
        FilesError::Closed => refused(
            Code::Unavailable,
            "The files tunnel closed: the session ended.".to_owned(),
        ),
        FilesError::Lost => refused(
            Code::Unavailable,
            "The files tunnel was lost: nothing came from the hub for 15 seconds.".to_owned(),
        ),
        FilesError::Tunnel(_) => refused(Code::Unavailable, error.to_string()),
        FilesError::Local(local) => Failure::Write {
            reason: local.to_string(),
        },
    }
}
