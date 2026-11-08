//! The share GraphQL part (K1 `shareSession`, `endShare`, `ShareInput`, `SessionShare`,
//! `ShareLink`; RA-035, D-7, D-9): merged into the root `Mutation` behind the same guard as the
//! sessions part. `Session.shares` is in the sessions part.

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ID, InputObject, Object, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    Error, Sharing,
    model::{SessionShare, ShareAccess, ShareLink},
    opening::share_session,
    terms::{DEFAULT_MINUTES, DEFAULT_VIEWERS, Terms, checked_terms},
};
use crate::{
    access::Authenticated,
    api_errors::{ErrorCode, coded, internal, invalid, schema_dependency},
    clock::Clock,
    config::Config,
    events::{Bus, ClientAddr, Operator},
    ids::{SessionId, ShareId},
    sessions::{Actor, FieldError, Live},
};

/// What the owner asks for; a null field means K1's default.
#[derive(InputObject)]
pub(super) struct ShareInput {
    #[graphql(default_with = "Some(ShareAccess::View)")]
    access: Option<ShareAccess>,
    /// 5..1440.
    #[graphql(default_with = "Some(DEFAULT_MINUTES)")]
    expires_in_minutes: Option<i32>,
    /// 1..10.
    #[graphql(default_with = "Some(DEFAULT_VIEWERS)")]
    max_viewers: Option<i32>,
}

impl ShareInput {
    fn into_terms(self) -> Result<Terms, Vec<FieldError>> {
        checked_terms(
            self.access.unwrap_or(ShareAccess::View),
            self.expires_in_minutes.unwrap_or(DEFAULT_MINUTES),
            self.max_viewers.unwrap_or(DEFAULT_VIEWERS),
        )
    }
}

#[Object]
impl SessionShare {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn session_id(&self) -> ID {
        ID(self.session.to_string())
    }

    async fn access(&self) -> ShareAccess {
        self.access
    }

    async fn max_viewers(&self) -> i32 {
        self.max_viewers.into()
    }

    /// Viewers connected now.
    async fn viewers(&self) -> i32 {
        self.viewers.into()
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

#[Object]
impl ShareLink {
    async fn share(&self) -> &SessionShare {
        &self.share
    }

    /// Shown once: `<public origin>/shared#<token>`. The token is only in the fragment (D-7).
    #[graphql(name = "url")]
    async fn url_field(&self) -> &str {
        self.url()
    }
}

/// The GraphQL error with its `extensions.code` (K7); a dependency failure shows no detail.
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::UnknownSession(id) => coded(format!("No such session {id}."), ErrorCode::NotFound),
        Error::UnknownShare(id) => coded(format!("No such share {id}."), ErrorCode::NotFound),
        Error::NotOwner => coded(
            "Only the operator who started this session can share it or end its shares.",
            ErrorCode::Forbidden,
        ),
        Error::NotOpen => coded(
            "This session has ended, or its window is not open yet. Share it while it is open.",
            ErrorCode::Conflict,
        ),
        Error::Unavailable => coded(
            "The hub cannot share a session now. End a share or try again in a moment.",
            ErrorCode::Unavailable,
        ),
        Error::Sessions(_) | Error::Events(_) | Error::Database(_) | Error::Random(_) => {
            internal(error)
        }
    }
}

/// The schema's stores, bus, clock and live state, for a share and its keeper.
fn sharing(ctx: &Context<'_>) -> Result<Sharing> {
    Ok(Sharing {
        db: schema_dependency::<PgPool>(ctx)?.clone(),
        bus: schema_dependency::<Bus>(ctx)?.clone(),
        clock: schema_dependency::<Arc<dyn Clock>>(ctx)?.clone(),
        live: schema_dependency::<Live>(ctx)?.clone(),
    })
}

/// `text` as an id, else VALIDATION on `field`.
fn parsed<T: std::str::FromStr>(text: &ID, field: &'static str) -> Result<T> {
    text.parse::<T>()
        .map_err(|_| invalid(&[FieldError::new(field, "That id is not valid.")]))
}

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`; the name only keeps it apart from the other parts.
#[Object(name = "ShareMutation")]
impl Mutation {
    /// Shares a live session the caller started, while its window is open (RA-035). The link
    /// is shown once.
    #[graphql(guard = "Authenticated")]
    async fn share_session(
        &self,
        ctx: &Context<'_>,
        session_id: ID,
        input: ShareInput,
    ) -> Result<ShareLink> {
        let session = parsed::<SessionId>(&session_id, "sessionId")?;
        let terms = input.into_terms().map_err(|fields| invalid(&fields))?;
        let sharing = sharing(ctx)?;
        let by = Actor {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            at: sharing.clock.now(),
        };
        let origin = &schema_dependency::<Arc<Config>>(ctx)?.public_origin;
        let (share, token) = share_session(&sharing, &by, session, terms)
            .await
            .map_err(to_graphql)?;
        Ok(ShareLink::new(share, origin, &token))
    }

    /// Ends a share of the caller's session; its viewers are let go.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn end_share(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let share = parsed::<ShareId>(&id, "id")?;
        let operator = schema_dependency::<Operator>(ctx)?;
        let client = *schema_dependency::<ClientAddr>(ctx)?;
        schema_dependency::<Live>(ctx)?
            .shares()
            .end_by(share, operator, client)
            .map_err(to_graphql)?;
        Ok(ID(share.to_string()))
    }
}
