//! The projects domain's nouns: the hosting kinds a project can use, the state of a proposal,
//! the signs a repository shows, and a repository's fields as the forge gives them.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use provider_github::{Origin, SignKind};

use crate::ids::{ProjectId, RepositoryId, ResourceId, SourceAccountId};

/// Where a project runs. Only hosting: a service that holds data about the operator is never
/// a resource (PRO-006).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
pub enum ResourceKind {
    Cloudflare,
    Vercel,
    Server,
    DesktopApplication,
}

impl ResourceKind {
    /// Every kind, in the order the API lists them.
    pub const ALL: [Self; 4] = [
        Self::Cloudflare,
        Self::Vercel,
        Self::Server,
        Self::DesktopApplication,
    ];

    /// The text stored in `resources.kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cloudflare => "cloudflare",
            Self::Vercel => "vercel",
            Self::Server => "server",
            Self::DesktopApplication => "desktop_application",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }
}

/// Whether the operator has confirmed a proposal (PRO-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ResourceState {
    /// Found in the repository; the hub removes it when its last sign goes.
    FoundInRepository,
    /// Confirmed; no sync removes it.
    Confirmed,
}

impl ResourceState {
    /// The text stored in `resources.state`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FoundInRepository => "found",
            Self::Confirmed => "confirmed",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::FoundInRepository, Self::Confirmed]
            .into_iter()
            .find(|state| state.as_str() == text)
    }
}

/// What a repository shows about where it runs (PRO-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Enum)]
#[graphql(name = "RepositorySign")]
pub enum Sign {
    WranglerConfig,
    VercelJson,
    VercelDeployment,
    Dockerfile,
    ComposeFile,
    TauriConf,
}

impl Sign {
    const ALL: [Self; 6] = [
        Self::WranglerConfig,
        Self::VercelJson,
        Self::VercelDeployment,
        Self::Dockerfile,
        Self::ComposeFile,
        Self::TauriConf,
    ];

    /// The text stored in `signs.sign`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WranglerConfig => "wrangler_config",
            Self::VercelJson => "vercel_json",
            Self::VercelDeployment => "vercel_deployment",
            Self::Dockerfile => "dockerfile",
            Self::ComposeFile => "compose_file",
            Self::TauriConf => "tauri_conf",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sign| sign.as_str() == text)
    }

    /// The forge's sign as the hub names it.
    #[must_use]
    pub const fn of(kind: SignKind) -> Self {
        match kind {
            SignKind::WranglerConfig => Self::WranglerConfig,
            SignKind::VercelJson => Self::VercelJson,
            SignKind::VercelDeployment => Self::VercelDeployment,
            SignKind::Dockerfile => Self::Dockerfile,
            SignKind::ComposeFile => Self::ComposeFile,
            SignKind::TauriConf => Self::TauriConf,
        }
    }
}

/// The text stored in `signs.origin`.
#[must_use]
pub const fn origin_text(origin: Origin) -> &'static str {
    match origin {
        Origin::Tree => "tree",
        Origin::Deployments => "deployments",
    }
}

/// A repository's fields as the forge gives them; the hub compares these, never `original`,
/// whose counts and stamps change on every listing (SRC-012).
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Canonical {
    pub full_name: String,
    pub html_url: String,
    pub default_branch: Option<String>,
    pub private: bool,
    pub archived: bool,
    pub fork: bool,
    pub description: Option<String>,
    pub pushed_at: Option<DateTime<Utc>>,
}

/// A stored repository, as the import rule reads it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Known {
    pub id: RepositoryId,
    pub project_id: ProjectId,
    pub account_id: Option<SourceAccountId>,
    #[sqlx(flatten)]
    pub fields: Canonical,
    pub left_out: bool,
    pub gone: bool,
}

/// A stored resource, as the proposals rule reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub state: ResourceState,
}
