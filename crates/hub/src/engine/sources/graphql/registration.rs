//! The Connect GitHub part of the API (K1; K-E1 Section 7a): `githubConnect`,
//! `startGithubConnect`, `openGithubConnect`, `cancelGithubConnect` and `githubConnectChanged`.
//!
//! The three mutations are closed to API keys by the access guard (D-23, R-100). The one-time
//! link is the only value an answer carries that opens anything, and only `openGithubConnect`
//! returns it, never to a request that declares an AI agent (PRJ-041, LOG-012). No answer holds
//! a key, a code, a cookie or a state (LOG-007).

use std::sync::Arc;

use async_graphql::{
    Context, ID, InputObject, Object, Result, SimpleObject, Subscription as SubscriptionObject,
};
use chrono::{DateTime, Utc};
use futures_util::{Stream, stream};
use ring::rand::SystemRandom;
use source_exchange::Registration;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use super::{failed, scheduling, seen, views::AccountView};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, schema_dependency,
    },
    config::Config,
    engine::sources::{
        Error, Reach, Scheduling, connect,
        registration::{
            self,
            attempt::{Attempt, Failure, Phase, Refusal, Request, Snapshot, Starter},
            binding::{Minted, name_entropy},
            trail, words,
        },
        store,
    },
    fleet::{
        access::Authenticated,
        events::{ActorKind, ClientAddr, Operator, Trail},
    },
};

/// The attempt as the API shows it, with what its links are built from.
pub struct AttemptView {
    snapshot: Snapshot,
    origin: Arc<str>,
    registration: Registration,
}

/// What every view of a request is built with: the hub's public origin and GitHub's pages.
#[derive(Clone)]
struct Pages {
    origin: Arc<str>,
    registration: Registration,
}

impl Pages {
    fn of(ctx: &Context<'_>, scheduling: &Scheduling) -> Result<Self> {
        let config = schema_dependency::<Arc<Config>>(ctx)?;
        let registration = registration::github(&scheduling.registry)
            .ok_or_else(|| failed(Error::UnknownSource))?;
        Ok(Self {
            origin: Arc::from(config.public_origin.as_str()),
            registration,
        })
    }

    fn view(&self, snapshot: Snapshot) -> AttemptView {
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
    url: String,
    expires_at: DateTime<Utc>,
}

/// The API's answer to a refused step.
fn refused(refusal: Refusal) -> async_graphql::Error {
    match refusal {
        Refusal::NotFound => coded(
            "No GitHub connection with this ID is open.",
            ErrorCode::NotFound,
        ),
        Refusal::AppWaiting => coded(
            "A GitHub App is made and waits for Install. Finish it or cancel it first.",
            ErrorCode::Conflict,
        ),
        _ => coded(
            "The GitHub connection is not at a step that allows this.",
            ErrorCode::Conflict,
        ),
    }
}

fn attempt_id(id: &ID) -> Result<Uuid> {
    id.parse().map_err(|_| refused(Refusal::NotFound))
}

/// The checked label and organisation of `input`.
fn checked(
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

/// The attempts slot of the schema.
fn attempts<'a>(ctx: &Context<'a>) -> Result<&'a registration::attempts::Attempts> {
    Ok(&schema_dependency::<Reach>(ctx)?.attempts)
}

#[derive(Default)]
pub struct Query;

#[Object(name = "GithubConnectQuery")]
impl Query {
    /// The open attempt, or one that ended in the last 10 minutes.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "async-graphql resolvers must be async; this one can fail"
    )]
    #[graphql(guard = "Authenticated")]
    async fn github_connect(&self, ctx: &Context<'_>) -> Result<Option<AttemptView>> {
        let s = scheduling(ctx)?;
        let pages = Pages::of(ctx, &s)?;
        let current = attempts(ctx)?.current(s.clock.now());
        Ok(current.map(|snapshot| pages.view(snapshot)))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "GithubConnectMutation")]
impl Mutation {
    /// Starts an attempt, in place of one that made no App.
    #[graphql(guard = "Authenticated")]
    async fn start_github_connect(
        &self,
        ctx: &Context<'_>,
        input: Option<StartGithubConnectInput>,
    ) -> Result<AttemptView> {
        let s = scheduling(ctx)?;
        let pages = Pages::of(ctx, &s)?;
        let (label, organisation) = checked(input.unwrap_or_default(), &pages.registration)?;
        let trail = Trail::from_context(ctx)?;
        let now = trail.now;
        let by = Starter {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
        };
        let entropy = name_entropy(&SystemRandom::new()).map_err(internal)?;
        let entry = trail::started(
            label.as_deref(),
            organisation.as_deref(),
            by.operator.actor.kind(),
        );
        let request = Request {
            label,
            organisation,
            name: (pages.registration.name)(&entropy),
            by,
        };
        let id = Uuid::now_v7();
        let slot = attempts(ctx)?;
        let snapshot = slot
            .start(Attempt::start(id, request, now), now)
            .map_err(refused)?;
        let mut tx = begin(ctx).await?;
        let written = match trail.write(&mut tx, entry).await {
            Ok(()) => commit(tx).await,
            Err(error) => Err(internal(error)),
        };
        if let Err(error) = written {
            slot.forget(id, now);
            return Err(error);
        }
        Ok(pages.view(snapshot))
    }

    /// A new one-time link for attempt `id`; an earlier one stops working.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "async-graphql resolvers must be async; this one can fail"
    )]
    #[graphql(guard = "Authenticated")]
    async fn open_github_connect(&self, ctx: &Context<'_>, id: ID) -> Result<Link> {
        let operator = schema_dependency::<Operator>(ctx)?;
        if operator.actor.kind() == ActorKind::Agent {
            return Err(coded(
                "An AI agent cannot open the one-time link. Give the operator the page link instead.",
                ErrorCode::Forbidden,
            ));
        }
        let s = scheduling(ctx)?;
        let pages = Pages::of(ctx, &s)?;
        let id = attempt_id(&id)?;
        let ticket = Minted::new(&SystemRandom::new()).map_err(internal)?;
        let now = s.clock.now();
        let expires_at = attempts(ctx)?
            .step(Some(id), now, |held| held.issue_ticket(ticket.hash(), now))
            .map_err(refused)?;
        let url = format!(
            "{}?ticket={}",
            words::page_url(&pages.origin),
            ticket.text()
        );
        Ok(Link { url, expires_at })
    }

    /// Cancels attempt `id` and drops any key it holds.
    #[graphql(guard = "Authenticated")]
    async fn cancel_github_connect(&self, ctx: &Context<'_>, id: ID) -> Result<AttemptView> {
        let s = scheduling(ctx)?;
        let pages = Pages::of(ctx, &s)?;
        let id = attempt_id(&id)?;
        let trail = Trail::from_context(ctx)?;
        let now = trail.now;
        let (was_open, snapshot) = attempts(ctx)?
            .step(Some(id), now, |held| {
                let was_open = !held.ended();
                held.cancel(now)?;
                Ok((was_open, held.snapshot()))
            })
            .map_err(refused)?;
        if was_open {
            let mut tx = begin(ctx).await?;
            trail
                .write(&mut tx, trail::cancelled())
                .await
                .map_err(internal)?;
            commit(tx).await?;
        }
        Ok(pages.view(snapshot))
    }
}

#[derive(Default)]
pub struct Subscription;

#[SubscriptionObject(name = "GithubConnectSubscription")]
impl Subscription {
    /// Each change of the attempt: a new one, each phase, a failure, a cancel.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn github_connect_changed(
        &self,
        ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = AttemptView>> {
        let s = scheduling(ctx)?;
        let pages = Pages::of(ctx, &s)?;
        let received = attempts(ctx)?.subscribe();
        Ok(stream::unfold(
            (received, pages),
            |(mut received, pages)| async move {
                loop {
                    match received.recv().await {
                        Ok(snapshot) => return Some((pages.view(snapshot), (received, pages))),
                        Err(RecvError::Lagged(_)) => {}
                        Err(RecvError::Closed) => return None,
                    }
                }
            },
        ))
    }
}
