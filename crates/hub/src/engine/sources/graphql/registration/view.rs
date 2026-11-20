//! What the Connect GitHub API shows and takes: the attempt with its links, the start input and
//! the one-time link. No key, code, cookie or state (LOG-007).

use std::sync::Arc;

use async_graphql::{Context, ID, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use source_exchange::Registration;

use super::super::{failed, scheduling, seen, views::AccountView};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, invalid, schema_dependency},
    config::Config,
    engine::sources::{
        Error, Scheduling, connect,
        registration::{
            self,
            attempt::{Failure, Phase, Snapshot},
            words,
        },
        store,
    },
    fleet::events::ActorKind,
};

const NO_BROWSER_ADDRESS: &str = "This hub has no address that a browser can open, so Connect GitHub \
    cannot start here. Use the key file of a GitHub App.";

/// Refuses the flow on a hub that no browser can open (HMD-070): a hub in the link mode that was
/// given no public origin. Its pages and the App that GitHub would make would name the default
/// origin, which is a different server. A hub of the pool is given the address of its gateway
/// (HMD-069); a hub on a server is reached at its own origin and is not in the link mode.
pub(super) fn browser_can_open(ctx: &Context<'_>) -> Result<()> {
    let config = schema_dependency::<Arc<Config>>(ctx)?;
    if config.link_socket.is_some() && !config.public_origin_given {
        return Err(coded(NO_BROWSER_ADDRESS, ErrorCode::Conflict));
    }
    Ok(())
}

/// The attempt as the API shows it, with what its links are built from.
pub struct AttemptView {
    snapshot: Snapshot,
    origin: Arc<str>,
    registration: Registration,
}

/// What every view of a request is built with: the hub's public origin and GitHub's pages.
#[derive(Clone)]
pub(super) struct Pages {
    pub(super) origin: Arc<str>,
    pub(super) registration: Registration,
}

impl Pages {
    pub(super) fn of(ctx: &Context<'_>, scheduling: &Scheduling) -> Result<Self> {
        let config = schema_dependency::<Arc<Config>>(ctx)?;
        let registration = registration::github(&scheduling.registry)
            .ok_or_else(|| failed(Error::UnknownSource))?;
        Ok(Self {
            origin: Arc::from(config.public_origin.as_str()),
            registration,
        })
    }

    pub(super) fn view(&self, snapshot: Snapshot) -> AttemptView {
        AttemptView {
            snapshot,
            origin: self.origin.clone(),
            registration: self.registration,
        }
    }
}

#[Object(name = "GithubConnectAttempt")]
impl AttemptView {
    async fn id(&self) -> ID {
        ID(self.snapshot.id.to_string())
    }

    async fn phase(&self) -> Phase {
        self.snapshot.phase
    }

    async fn started_by(&self) -> ActorKind {
        self.snapshot.started_by
    }

    async fn started_at(&self) -> DateTime<Utc> {
        self.snapshot.started_at
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.snapshot.expires_at
    }

    async fn page_url(&self) -> String {
        words::page_url(&self.origin)
    }

    async fn app_id(&self) -> Option<String> {
        self.snapshot.app.as_ref().map(|app| app.id.to_string())
    }

    async fn app_slug(&self) -> Option<&str> {
        self.snapshot.app.as_ref().map(|app| app.slug.as_str())
    }

    async fn owner_login(&self) -> Option<&str> {
        self.snapshot
            .app
            .as_ref()
            .map(|app| app.owner.login.as_str())
    }

    async fn install_url(&self) -> Option<String> {
        words::install_url(&self.snapshot, &self.registration)
    }

    async fn app_settings_url(&self) -> Option<String> {
        words::settings_url(&self.snapshot, &self.registration)
    }

    async fn failure(&self) -> Option<Failure> {
        self.snapshot.failure
    }

    async fn message(&self) -> Option<String> {
        Some(words::message(&self.snapshot))
    }

    /// The account, once connected; `None` after it was removed.
    async fn account(&self, ctx: &Context<'_>) -> Result<Option<AccountView>> {
        let Some(id) = self.snapshot.account else {
            return Ok(None);
        };
        let s = scheduling(ctx)?;
        match store::account(&s.pool, id).await.map_err(failed)? {
            Some(account) => Ok(Some(seen(&s, account).await?.0)),
            None => Ok(None),
        }
    }
}

/// What the operator asks for at the start.
#[derive(Default, InputObject)]
#[graphql(name = "StartGithubConnectInput")]
pub struct StartGithubConnectInput {
    /// 1 to 64 characters. Null: the login of the App's owner.
    label: Option<String>,
    /// A GitHub organisation login. Null: the operator's own account.
    organisation: Option<String>,
}

/// ONE-TIME. Never logged, never in the audit log, never returned by an MCP tool (MCP-004).
#[derive(SimpleObject)]
#[graphql(name = "GithubConnectLink")]
pub struct Link {
    pub(super) url: String,
    pub(super) expires_at: DateTime<Utc>,
}

/// The checked label and organisation of `input`.
pub(super) fn checked(
    input: StartGithubConnectInput,
    registration: &Registration,
) -> Result<(Option<String>, Option<String>)> {
    let label = match input.label {
        Some(text) => Some(connect::label(&text).ok_or_else(|| failed(Error::Label))?),
        None => None,
    };
    let organisation = input.organisation.map(|login| login.trim().to_owned());
    if organisation
        .as_deref()
        .is_some_and(|login| (registration.form_path)(Some(login)).is_none())
    {
        return Err(invalid(&[FieldError::new(
            "organisation",
            "Give the login of a GitHub organisation.",
        )]));
    }
    Ok((label, organisation))
}
