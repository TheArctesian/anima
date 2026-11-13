//! Why the command line tool was not installed or removed, as the client sees it: a stable code
//! and fixed text (INS-006). Paths are not secrets, but only the file that blocks an install is
//! named, and never in a log (LOG-007).

use std::path::PathBuf;

use serde::{Serialize, Serializer, ser::SerializeStruct};

/// What is at `<folder>/anima` when it is not this app's link or launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Occupant {
    File,
    Folder,
    /// A link to somewhere other than an `anima` in an app bundle.
    Link(PathBuf),
    /// A device file, socket or pipe.
    Other,
}

impl Occupant {
    fn kind(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Link(_) => "link",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandLineError {
    #[error("only the main window may install the command line tool")]
    Caller,
    #[error("the command line tool cannot be installed on this system")]
    Unsupported,
    #[error("this build of the app carries no command line tool")]
    NotBundled,
    #[error("the app runs from a disk image; move it to Applications first")]
    DiskImage,
    #[error(
        "macOS runs the app from a temporary place (App Translocation); move it to Applications first"
    )]
    Translocated,
    #[error("the app is not running from an AppImage")]
    NotAppImage,
    #[error("the path of the AppImage has a character the launcher cannot hold")]
    Unquotable,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error("the proposed folder changed; look at the new one and confirm it")]
    Changed,
    #[error("something that the app did not make is already there")]
    Occupied(Occupant),
    #[error("the folder could not be made or written")]
    Unwritable,
    #[error("the command line tool could not be changed")]
    Failed,
}

impl CommandLineError {
    /// Stable code the client switches on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Unsupported => "unsupported",
            Self::NotBundled => "not-bundled",
            Self::DiskImage => "disk-image",
            Self::Translocated => "translocated",
            Self::NotAppImage => "not-appimage",
            Self::Unquotable => "unquotable",
            Self::NoHome => "no-home",
            Self::Changed => "changed",
            Self::Occupied(_) => "occupied",
            Self::Unwritable => "unwritable",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code", "message" }`; `occupied` adds `occupant` (`file`, `folder`, `link`
/// or `other`) and, for a link, `linkTarget`.
impl Serialize for CommandLineError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("CommandLineError", 4)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        if let Self::Occupied(occupant) = self {
            out.serialize_field("occupant", occupant.kind())?;
            if let Occupant::Link(target) = occupant {
                out.serialize_field("linkTarget", &target.to_string_lossy())?;
            }
        }
        out.end()
    }
}
