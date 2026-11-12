use serde::{Serialize, Serializer, ser::SerializeStruct};

/// Why a build step did not finish. Carries no process output, path or secret: the step's lines went
/// to the web view as events, and only the variant crosses here (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BuilderError {
    #[error("another build step is running")]
    Busy,
    #[error("the commit is not 40 lower-case hex digits")]
    Commit,
    #[error("the version is not three numbers with dots between")]
    Version,
    #[error("the target is not one Anima builds")]
    Target,
    #[error("this computer cannot build that target")]
    Platform,
    #[error("the commit is not on main of the Anima repository")]
    NotOnMain,
    #[error("the working tree is not clean after the checkout")]
    Dirty,
    #[error("a tool the step needs is missing")]
    ToolMissing,
    #[error("the step failed; its log says why")]
    StepFailed,
    #[error("the step was cancelled")]
    Cancelled,
    #[error("the bundle is not where the build puts it")]
    Bundle,
    #[error("only the main window may run build steps")]
    Caller,
    #[error("the file could not be uploaded")]
    Upload,
    #[error("the builder could not start the step")]
    Failed,
}

impl BuilderError {
    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Commit => "commit",
            Self::Version => "version",
            Self::Target => "target",
            Self::Platform => "platform",
            Self::NotOnMain => "not-on-main",
            Self::Dirty => "dirty",
            Self::ToolMissing => "tool-missing",
            Self::StepFailed => "step-failed",
            Self::Cancelled => "cancelled",
            Self::Bundle => "bundle",
            Self::Caller => "caller",
            Self::Upload => "upload",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for BuilderError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("BuilderError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
