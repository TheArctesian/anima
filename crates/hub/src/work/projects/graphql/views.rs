//! The K1 shapes of a project, its repositories and its resources, built from the stored rows
//! of a set of projects. Pure.

use std::collections::HashMap;

use async_graphql::{ID, SimpleObject};
use chrono::{DateTime, Utc};

use crate::{
    ids::ProjectId,
    paging::PageEnd,
    work::projects::{
        model::{ResourceKind, ResourceState, Sign},
        page::{After, encode_cursor},
        proposals::kind_of,
        store::reads::{EvidenceRow, ProjectRow, RepositoryRow, ResourceRow},
    },
};

/// Where "Open in GitHub" goes when the stored address is not `https`.
const FORGE_ORIGIN: &str = "https://github.com/";

#[derive(SimpleObject)]
#[graphql(name = "ProjectRepository")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "K1 fixes the fields of ProjectRepository"
)]
pub(super) struct RepositoryView {
    id: ID,
    /// owner/name (PRJ-016).
    full_name: String,
    url: String,
    default_branch: Option<String>,
    private: bool,
    archived: bool,
    fork: bool,
    description: Option<String>,
    pushed_at: Option<DateTime<Utc>>,
    left_out: bool,
    /// True: the connection no longer lists it. The record is kept.
    gone: bool,
}

#[derive(SimpleObject)]
#[graphql(name = "ResourceEvidence")]
pub(super) struct EvidenceView {
    sign: Sign,
    /// Null for a deployment record.
    path: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "ProjectResource")]
pub(super) struct ResourceView {
    id: ID,
    kind: ResourceKind,
    /// `FOUND_IN_REPOSITORY` until the operator confirms it (PRO-004).
    state: ResourceState,
    evidence: Vec<EvidenceView>,
    found_at: DateTime<Utc>,
    confirmed_at: Option<DateTime<Utc>>,
}

#[derive(SimpleObject)]
#[graphql(name = "Project")]
pub(super) struct ProjectView {
    id: ID,
    slug: String,
    name: String,
    /// The full name of its first repository; other extensions keep this text (PRJ-016,
    /// LNK-004).
    subject_key: String,
    repositories: Vec<RepositoryView>,
    resources: Vec<ResourceView>,
    /// When the hub last read its repositories.
    synced_at: Option<DateTime<Utc>>,
}

#[derive(SimpleObject)]
#[graphql(name = "ProjectEdge")]
pub(super) struct ProjectEdge {
    cursor: String,
    node: ProjectView,
}

#[derive(SimpleObject)]
#[graphql(name = "ProjectConnection")]
pub(super) struct ProjectConnection {
    edges: Vec<ProjectEdge>,
    page_info: PageEnd,
}

/// The stored rows of a set of projects.
#[derive(Default)]
pub(super) struct Rows {
    pub repositories: Vec<RepositoryRow>,
    pub resources: Vec<ResourceRow>,
    pub evidence: Vec<EvidenceRow>,
}

fn repository(row: RepositoryRow) -> RepositoryView {
    let url = if row.html_url.starts_with("https://") {
        row.html_url
    } else {
        format!("{FORGE_ORIGIN}{}", row.full_name)
    };
    RepositoryView {
        id: ID(row.id.to_string()),
        full_name: row.full_name,
        url,
        default_branch: row.default_branch,
        private: row.private,
        archived: row.archived,
        fork: row.fork,
        description: row.description,
        pushed_at: row.pushed_at,
        left_out: row.left_out,
        gone: row.gone,
    }
}

/// `row` with the signs of its project that point at its kind; `None` for a row this version
/// cannot read.
pub(super) fn resource(row: &ResourceRow, evidence: &[EvidenceRow]) -> Option<ResourceView> {
    let kind = ResourceKind::parse(&row.kind)?;
    let evidence = evidence
        .iter()
        .filter(|(project, _, _)| *project == row.project_id)
        .filter_map(|(_, sign, path)| Some((Sign::parse(sign)?, path)))
        .filter(|(sign, _)| kind_of(*sign) == kind)
        .map(|(sign, path)| EvidenceView {
            sign,
            path: path.clone(),
        })
        .collect();
    Some(ResourceView {
        id: ID(row.id.to_string()),
        kind,
        state: ResourceState::parse(&row.state)?,
        evidence,
        found_at: row.found_at,
        confirmed_at: row.confirmed_at,
    })
}

/// The K1 projects of `projects`, in their order, from `rows`.
pub(super) fn projects(projects: Vec<ProjectRow>, rows: Rows) -> Vec<(After, ProjectView)> {
    let mut repositories: HashMap<ProjectId, Vec<RepositoryRow>> = HashMap::new();
    for row in rows.repositories {
        repositories.entry(row.project_id).or_default().push(row);
    }
    let mut resources: HashMap<ProjectId, Vec<ResourceView>> = HashMap::new();
    for row in rows.resources {
        let project = row.project_id;
        if let Some(view) = resource(&row, &rows.evidence) {
            resources.entry(project).or_default().push(view);
        }
    }
    projects
        .into_iter()
        .map(|project| {
            let held = repositories.remove(&project.id).unwrap_or_default();
            let after = After {
                name: project.sort_name,
                id: project.id,
            };
            let view = ProjectView {
                id: ID(project.id.to_string()),
                subject_key: held
                    .first()
                    .map_or_else(|| project.name.clone(), |r| r.full_name.clone()),
                synced_at: held.iter().filter_map(|r| r.listed_at).max(),
                slug: project.slug,
                name: project.name,
                repositories: held.into_iter().map(repository).collect(),
                resources: resources.remove(&project.id).unwrap_or_default(),
            };
            (after, view)
        })
        .collect()
}

/// A page of projects.
pub(super) fn connection(
    page: Vec<(After, ProjectView)>,
    has_next_page: bool,
) -> ProjectConnection {
    let edges: Vec<ProjectEdge> = page
        .into_iter()
        .map(|(after, node)| ProjectEdge {
            cursor: encode_cursor(&after),
            node,
        })
        .collect();
    let end_cursor = edges.last().map(|edge| edge.cursor.clone());
    ProjectConnection {
        edges,
        page_info: PageEnd {
            has_next_page,
            end_cursor,
        },
    }
}
