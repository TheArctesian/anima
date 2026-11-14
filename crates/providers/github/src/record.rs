//! What the GitHub provider hands the Projects intake (PRJ-016, PRO-003).

use chrono::{DateTime, Utc};
use serde_json::Value;

/// A repository as GitHub lists it. The subject key is `full_name` and follows a rename;
/// `external_id` is GitHub's numeric repository id and never changes (PRJ-016).
#[derive(Clone, Debug, PartialEq)]
pub struct Repository {
    pub external_id: String,
    pub full_name: String,
    pub html_url: String,
    pub default_branch: Option<String>,
    pub private: bool,
    pub archived: bool,
    pub fork: bool,
    pub description: Option<String>,
    pub pushed_at: Option<DateTime<Utc>>,
    /// The entry as GitHub sent it, without U+0000 (SRC-011).
    pub original: Value,
}

/// Where a sign was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Tree,
    Deployments,
}

/// What a repository shows about where it runs. The order is the order of the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignKind {
    WranglerConfig,
    VercelJson,
    VercelDeployment,
    Dockerfile,
    ComposeFile,
    TauriConf,
}

/// One sign. `path` is `None` for a deployment record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sign {
    pub kind: SignKind,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    Repository(Repository),
    /// Emitted once, when the listing is complete: every repository the connection can see.
    Listed {
        external_ids: Vec<String>,
    },
    /// All signs of one origin for one repository. Replaces what was stored for that origin.
    Signs {
        external_id: String,
        origin: Origin,
        signs: Vec<Sign>,
    },
}
