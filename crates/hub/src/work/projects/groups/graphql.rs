//! The K1 part of project groups (PRO-059 to PRO-063): `projectGroups`, `createProjectGroup`,
//! `renameProjectGroup`, `deleteProjectGroup` and `setProjectsGroup`. Each change writes its
//! rows and one audit entry in one transaction; the words name projects by slug and a group by
//! its id, never by its name, which the operator typed (LOG-007).

use async_graphql::{Context, ID, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use serde_json::json;

use super::{rules, store};
use crate::{
    api_errors::{FieldError, begin, commit, internal, invalid},
    fleet::{
        access::Authenticated,
        events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
    },
    ids::{ProjectGroupId, ProjectId},
    work::projects::{Error, on},
};

/// A group as K1 shows it.
#[derive(Clone, SimpleObject)]
#[graphql(name = "ProjectGroup")]
pub(in crate::work::projects) struct GroupView {
    id: ID,
    name: String,
    /// The projects of the group that have a repository that is not left out (PRO-064).
    project_count: i32,
    created_at: DateTime<Utc>,
}

impl From<store::GroupRow> for GroupView {
    fn from(row: store::GroupRow) -> Self {
        Self {
            id: ID(row.id.to_string()),
            name: row.name,
            project_count: i32::try_from(row.project_count).unwrap_or(i32::MAX),
            created_at: row.created_at,
        }
    }
}

fn failed(error: impl Into<Error>) -> async_graphql::Error {
    error.into().into_graphql()
}

fn group_id(id: &ID) -> Result<ProjectGroupId> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That id is not valid.")]))
}

fn checked_name(text: &str) -> Result<String> {
    rules::name(text).ok_or_else(|| {
        invalid(&[FieldError::new(
            "name",
            "Give a name of 1 to 60 characters with no control character.",
        )])
    })
}

fn project_ids(given: &[ID]) -> Result<Vec<ProjectId>> {
    let parsed: Vec<ProjectId> = given
        .iter()
        .map(|id| id.parse())
        .collect::<Result<_, _>>()
        .map_err(|_| invalid(&[FieldError::new("projects", "An id is not valid.")]))?;
    rules::distinct(&parsed).ok_or_else(|| {
        invalid(&[FieldError::new(
            "projects",
            "Give at most 200 projects in one change.",
        )])
    })
}

fn entry(
    action: AuditAction,
    group: ProjectGroupId,
    message: &str,
    change: serde_json::Value,
) -> TrailEntry {
    TrailEntry {
        source: Source::Hub,
        severity: Severity::Info,
        device: None,
        message: message.to_owned(),
        action,
        target: AuditTarget::ProjectGroup(group),
        change,
    }
}

/// Moves `projects` into `group` (or into no group) in `tx`; `NOT_FOUND` when a project does not
/// exist. Returns the audit change, or `None` when no project moved.
async fn moved(
    tx: &mut sqlx::PgConnection,
    projects: &[ProjectId],
    group: Option<ProjectGroupId>,
) -> Result<Option<(ProjectGroupId, serde_json::Value)>> {
    let found = store::existing(tx, projects).await.map_err(failed)?;
    if found.len() != projects.len() {
        return Err(failed(Error::NoProject));
    }
    let rows = store::set_group(tx, projects, group)
        .await
        .map_err(failed)?;
    let Some(target) = group.or_else(|| rows.iter().find_map(|row| row.before)) else {
        return Ok(None);
    };
    let each: Vec<_> = rows
        .iter()
        .map(|row| {
            json!({ "project": row.id.to_string(), "slug": row.slug,
                "from": row.before.map(|g| g.to_string()) })
        })
        .collect();
    let change = json!({ "group": group.map(|g| g.to_string()), "projects": each });
    Ok(Some((target, change)))
}

async fn shown(tx: &mut sqlx::PgConnection, id: ProjectGroupId) -> Result<GroupView> {
    store::one(tx, id)
        .await
        .map_err(failed)?
        .map(GroupView::from)
        .ok_or_else(|| failed(Error::NoGroup))
}

#[derive(Default)]
pub struct GroupsQuery;

#[Object(name = "ProjectGroupsQuery")]
impl GroupsQuery {
    /// Each group, in name order (PRO-059).
    #[graphql(guard = "Authenticated")]
    async fn project_groups(&self, ctx: &Context<'_>) -> Result<Vec<GroupView>> {
        let db = on(ctx).await?;
        let rows = store::all(db).await.map_err(failed)?;
        Ok(rows.into_iter().map(GroupView::from).collect())
    }
}

#[derive(Default)]
pub struct GroupsMutation;

#[Object(name = "ProjectGroupsMutation")]
impl GroupsMutation {
    /// Makes a group and puts `projects` in it; a project in a different group moves (PRO-060).
    #[graphql(guard = "Authenticated")]
    async fn create_project_group(
        &self,
        ctx: &Context<'_>,
        name: String,
        #[graphql(default)] projects: Vec<ID>,
    ) -> Result<GroupView> {
        on(ctx).await?;
        let name = checked_name(&name)?;
        let projects = project_ids(&projects)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        if store::name_taken(&mut tx, &name, None)
            .await
            .map_err(failed)?
        {
            return Err(failed(Error::GroupNameTaken));
        }
        let id = ProjectGroupId::from(uuid::Uuid::now_v7());
        store::insert(&mut tx, (id, &name), trail.now)
            .await
            .map_err(failed)?;
        let change = moved(&mut tx, &projects, Some(id))
            .await?
            .map(|(_, change)| change);
        trail
            .write(
                &mut tx,
                entry(
                    AuditAction::Create,
                    id,
                    "A project group was made.",
                    json!({ "moved": change }),
                ),
            )
            .await
            .map_err(internal)?;
        let view = shown(&mut tx, id).await?;
        commit(tx).await?;
        Ok(view)
    }

    /// `CONFLICT` when a different group has the name (PRO-059, PRO-062).
    #[graphql(guard = "Authenticated")]
    async fn rename_project_group(
        &self,
        ctx: &Context<'_>,
        id: ID,
        name: String,
    ) -> Result<GroupView> {
        on(ctx).await?;
        let id = group_id(&id)?;
        let name = checked_name(&name)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        if !store::lock(&mut tx, id).await.map_err(failed)? {
            return Err(failed(Error::NoGroup));
        }
        if store::name_taken(&mut tx, &name, Some(id))
            .await
            .map_err(failed)?
        {
            return Err(failed(Error::GroupNameTaken));
        }
        store::rename(&mut tx, id, &name).await.map_err(failed)?;
        trail
            .write(
                &mut tx,
                entry(
                    AuditAction::Update,
                    id,
                    "A project group was renamed.",
                    json!({}),
                ),
            )
            .await
            .map_err(internal)?;
        let view = shown(&mut tx, id).await?;
        commit(tx).await?;
        Ok(view)
    }

    /// Deletes the group only: each of its projects stays, with no group, and none is left out
    /// (PRO-062). Returns the id.
    #[graphql(guard = "Authenticated")]
    async fn delete_project_group(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        on(ctx).await?;
        let group = group_id(&id)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        if !store::lock(&mut tx, group).await.map_err(failed)? {
            return Err(failed(Error::NoGroup));
        }
        store::delete(&mut tx, group).await.map_err(failed)?;
        trail
            .write(
                &mut tx,
                entry(
                    AuditAction::Delete,
                    group,
                    "A project group was deleted.",
                    json!({}),
                ),
            )
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }

    /// Puts `projects` in `group`, or in no group when `group` is null (PRO-060, PRO-063).
    /// Returns the ids of the projects that moved.
    #[graphql(guard = "Authenticated")]
    async fn set_projects_group(
        &self,
        ctx: &Context<'_>,
        projects: Vec<ID>,
        group: Option<ID>,
    ) -> Result<Vec<ID>> {
        on(ctx).await?;
        let projects = project_ids(&projects)?;
        let group = group.as_ref().map(group_id).transpose()?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        if let Some(group) = group
            && !store::lock(&mut tx, group).await.map_err(failed)?
        {
            return Err(failed(Error::NoGroup));
        }
        let Some((target, change)) = moved(&mut tx, &projects, group).await? else {
            drop(trail);
            commit(tx).await?;
            return Ok(Vec::new());
        };
        let ids: Vec<ID> = change["projects"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|each| each["project"].as_str().map(|id| ID(id.to_owned())))
            .collect();
        let words = if group.is_some() {
            "Projects were put in a group."
        } else {
            "Projects were removed from a group."
        };
        trail
            .write(&mut tx, entry(AuditAction::Update, target, words, change))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(ids)
    }
}
