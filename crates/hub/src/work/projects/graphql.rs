//! The projects GraphQL part (K1; XP-02): the projects, one project, leave out and restore a
//! repository, confirm a resource.
//!
//! Every field is `CONFLICT` while Projects is off (EXT-002). Each mutation that changes
//! something writes its change, one audit entry and one event in one transaction.

mod trail;
mod views;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};
use sqlx::{PgConnection, PgPool};

use self::views::{ProjectConnection, ProjectView, ResourceView, Rows};
use super::{
    Error,
    model::ResourceKind,
    page::{After, decode_cursor, page_size},
    restate,
    store::{choices, reads},
};
use crate::{
    api_errors::{FieldError, begin, commit, internal, invalid, pool},
    engine::{
        extensions::{self, PROJECTS},
        sources,
    },
    fleet::{access::Authenticated, events::Trail},
    ids::{ProjectId, RepositoryId, ResourceId, SourceAccountId},
};

/// K1's default for `first`.
const DEFAULT_FIRST: i32 = 50;
/// The longest slug the hub makes.
const MAX_SLUG: usize = 120;

fn failed(error: impl Into<Error>) -> GraphQlError {
    error.into().into_graphql()
}

/// The pool, once Projects is on; `CONFLICT` while it is off.
async fn on<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    let db = pool(ctx)?;
    extensions::require_on(db, PROJECTS)
        .await
        .map_err(extensions::Error::into_graphql)?;
    Ok(db)
}

fn parsed<T: std::str::FromStr>(id: &ID) -> Result<T> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That id is not valid.")]))
}

/// The K1 views of `projects`, with their repositories and resources.
async fn shown(db: &PgPool, projects: Vec<reads::ProjectRow>) -> Result<Vec<(After, ProjectView)>> {
    let ids: Vec<ProjectId> = projects.iter().map(|p| p.id).collect();
    let rows = Rows {
        repositories: reads::repositories(db, &ids).await.map_err(failed)?,
        resources: reads::resources(db, &ids).await.map_err(failed)?,
        evidence: reads::evidence(db, &ids).await.map_err(failed)?,
    };
    Ok(views::projects(projects, rows))
}

/// The project `id` as K1 shows it.
async fn project_view(db: &PgPool, id: ProjectId) -> Result<ProjectView> {
    let row = reads::by_id(db, id).await.map_err(failed)?;
    let shown = shown(db, row.into_iter().collect()).await?;
    shown
        .into_iter()
        .next()
        .map(|(_, view)| view)
        .ok_or_else(|| failed(Error::NoRepository))
}

/// Makes `account` due now; an account removed meanwhile has nothing to make due.
async fn make_due(conn: &mut PgConnection, account: SourceAccountId) -> Result<()> {
    match sources::request_sync(conn, account).await {
        Ok(()) | Err(sources::Error::NotFound) => Ok(()),
        Err(e) => Err(failed(Error::Sync(e))),
    }
}

/// Before a restore locks the repository: makes its account due, which locks the account row
/// first, as a sync page does, so the two never deadlock. Returns the account made due.
async fn due_first(conn: &mut PgConnection, id: RepositoryId) -> Result<Option<SourceAccountId>> {
    let peeked = choices::peek(conn, id).await.map_err(failed)?;
    let Some(account) = peeked.filter(|p| p.left_out).and_then(|p| p.account_id) else {
        return Ok(None);
    };
    make_due(conn, account).await?;
    Ok(Some(account))
}

/// Leaves the repository `id` out, or restores it; a restore makes its account due, so its
/// signs are read again on the next sync (PRO-002). No change when it is already so.
async fn choose(ctx: &Context<'_>, id: &ID, left_out: bool) -> Result<ProjectView> {
    let db = on(ctx).await?;
    let id: RepositoryId = parsed(id)?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let due = if left_out {
        None
    } else {
        due_first(&mut tx, id).await?
    };
    let chosen = choices::repository(&mut tx, id)
        .await
        .map_err(failed)?
        .ok_or_else(|| failed(Error::NoRepository))?;
    let project = chosen.project_id;
    if chosen.left_out == left_out {
        drop(trail);
    } else {
        let now = trail.now;
        choices::set_left_out(&mut tx, id, left_out.then_some(now))
            .await
            .map_err(failed)?;
        restate(&mut tx, project, now).await.map_err(failed)?;
        // Only when another choice changed the repository between the peek and the lock.
        if let (false, Some(account)) = (left_out, chosen.account_id.filter(|a| due != Some(*a))) {
            make_due(&mut tx, account).await?;
        }
        let slug = choices::slug(&mut tx, project).await.map_err(failed)?;
        trail
            .write(&mut tx, trail::left_out((project, &slug), id, left_out))
            .await
            .map_err(internal)?;
    }
    commit(tx).await?;
    project_view(db, project).await
}

#[derive(Default)]
pub struct Query;

#[Object(name = "ProjectsQuery")]
impl Query {
    /// By name. `leftOut` false: the projects with a repository that is not left out; true:
    /// the projects whose repositories are all left out.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.projects"
    )]
    async fn projects(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(false)")] left_out: Option<bool>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<ProjectConnection> {
        let db = on(ctx).await?;
        let size = page_size(first.unwrap_or(DEFAULT_FIRST));
        let after = after.as_deref().map(decode_cursor).transpose();
        let (size, after) = match (size, after) {
            (Ok(size), Ok(after)) => (size, after),
            (size, after) => {
                let errors: Vec<FieldError> = size.err().into_iter().chain(after.err()).collect();
                return Err(invalid(&errors));
            }
        };
        let limit = i64::try_from(size + 1).unwrap_or(i64::MAX);
        let mut rows = reads::page(db, left_out.unwrap_or(false), after.as_ref(), limit)
            .await
            .map_err(failed)?;
        let has_next_page = rows.len() > size;
        rows.truncate(size);
        Ok(views::connection(shown(db, rows).await?, has_next_page))
    }

    #[graphql(guard = "Authenticated")]
    async fn project(&self, ctx: &Context<'_>, slug: String) -> Result<Option<ProjectView>> {
        let db = on(ctx).await?;
        if slug.len() > MAX_SLUG {
            return Ok(None);
        }
        let row = reads::by_slug(db, &slug).await.map_err(failed)?;
        let shown = shown(db, row.into_iter().collect()).await?;
        Ok(shown.into_iter().next().map(|(_, view)| view))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "ProjectsMutation")]
impl Mutation {
    /// The hub does not import or read the repository again until it is restored (PRO-002).
    #[graphql(guard = "Authenticated")]
    async fn leave_out_repository(&self, ctx: &Context<'_>, id: ID) -> Result<ProjectView> {
        choose(ctx, &id, true).await
    }

    /// Its signs are read again on the next sync, which this makes due now.
    #[graphql(guard = "Authenticated")]
    async fn restore_repository(&self, ctx: &Context<'_>, id: ID) -> Result<ProjectView> {
        choose(ctx, &id, false).await
    }

    /// `CONFLICT` when it is already confirmed (PRO-004).
    #[graphql(guard = "Authenticated")]
    async fn confirm_project_resource(&self, ctx: &Context<'_>, id: ID) -> Result<ResourceView> {
        let db = on(ctx).await?;
        let id: ResourceId = parsed(&id)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (project, kind, state) = choices::resource(&mut tx, id)
            .await
            .map_err(failed)?
            .ok_or_else(|| failed(Error::NoResource))?;
        if state == "confirmed" {
            return Err(failed(Error::AlreadyConfirmed));
        }
        choices::confirm(&mut tx, id, trail.now)
            .await
            .map_err(failed)?;
        let slug = choices::slug(&mut tx, project).await.map_err(failed)?;
        let kind = ResourceKind::parse(&kind).ok_or_else(|| failed(Error::NoResource))?;
        trail
            .write(&mut tx, trail::confirmed((project, &slug), id, kind))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        let evidence = reads::evidence(db, &[project]).await.map_err(failed)?;
        reads::resources(db, &[project])
            .await
            .map_err(failed)?
            .into_iter()
            .find(|row| row.id == id)
            .and_then(|row| views::resource(&row, &evidence))
            .ok_or_else(|| failed(Error::NoResource))
    }
}
