import { graphql } from '$lib/hub/generated';
import type { ProjectBrowse } from '$lib/hub/generated/graphql';
export type { ProjectBrowse } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';
import { groupsApi, type GroupsApi } from './groups/api';

// The fragments are only read by codegen, which inlines them into the documents that spread them.
void graphql(`
  fragment ResourceFields on ProjectResource {
    id
    kind
    state
    foundAt
    confirmedAt
    evidence {
      sign
      path
    }
  }
`);

void graphql(`
  fragment ProjectFields on Project {
    id
    slug
    name
    subjectKey
    syncedAt
    repositories {
      id
      fullName
      url
      defaultBranch
      private
      archived
      fork
      description
      leftOut
      gone
    }
    resources {
      ...ResourceFields
    }
    group {
      id
      name
    }
  }
`);

const PROJECTS = graphql(`
  query Projects($leftOut: Boolean!, $first: Int!, $after: String, $browse: ProjectBrowse) {
    projects(leftOut: $leftOut, first: $first, after: $after, browse: $browse) {
      languages
      edges {
        node {
          ...ProjectFields
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
`);

const PROJECT = graphql(`
  query Project($slug: String!) {
    project(slug: $slug) {
      ...ProjectFields
    }
  }
`);

const LEAVE_OUT = graphql(`
  mutation LeaveOutRepository($id: ID!) {
    leaveOutRepository(id: $id) {
      ...ProjectFields
    }
  }
`);

const RESTORE = graphql(`
  mutation RestoreRepository($id: ID!) {
    restoreRepository(id: $id) {
      ...ProjectFields
    }
  }
`);

const CONFIRM = graphql(`
  mutation ConfirmProjectResource($id: ID!) {
    confirmProjectResource(id: $id) {
      ...ResourceFields
    }
  }
`);

export type ResourceKind = 'CLOUDFLARE' | 'VERCEL' | 'SERVER' | 'DESKTOP_APPLICATION';
export type ResourceState = 'FOUND_IN_REPOSITORY' | 'CONFIRMED';
export type RepositorySign = 'WRANGLER_CONFIG' | 'VERCEL_JSON' | 'VERCEL_DEPLOYMENT' | 'DOCKERFILE' | 'COMPOSE_FILE' | 'TAURI_CONF';

export interface ProjectRepository {
  readonly id: string;
  /** owner/name (PRJ-016). */
  readonly fullName: string;
  readonly url: string;
  /** The branch Code and History open on, and a workflow starts on by default. */
  readonly defaultBranch?: string | null;
  readonly description?: string | null;
  readonly private: boolean;
  readonly archived: boolean;
  readonly fork: boolean;
  readonly leftOut: boolean;
  /** The connection no longer lists it; the record is kept. */
  readonly gone: boolean;
}

export interface ResourceEvidence {
  readonly sign: RepositorySign;
  /** Null for a deployment record. */
  readonly path?: string | null;
}

export interface ProjectResource {
  readonly id: string;
  readonly kind: ResourceKind;
  readonly state: ResourceState;
  readonly evidence: readonly ResourceEvidence[];
}

export interface Project {
  readonly id: string;
  readonly slug: string;
  readonly name: string;
  readonly repositories: readonly ProjectRepository[];
  readonly resources: readonly ProjectResource[];
  /** The operator's group of the project (PRO-059); none when it is in no group. */
  readonly group?: { readonly id: string; readonly name: string } | null;
}

/** One page of projects, with where the next one starts (null: no more). */
export interface ProjectsPage {
  readonly projects: readonly Project[];
  readonly next: string | null;
  readonly languages?: readonly string[];
}

/** The project calls of the hub (PRO-001, PRO-002, PRO-004) and its group calls (PRO-059 to PRO-063). */
export interface ProjectsApi extends GroupsApi {
  /** One page in the requested order; `leftOut` true lists the projects whose repositories are all left out. */
  list(leftOut: boolean, after: string | null, browse?: ProjectBrowse): Promise<HubResult<ProjectsPage>>;
  /** Null when no project has the slug. */
  find(slug: string): Promise<HubResult<Project | null>>;
  leaveOut(repositoryId: string): Promise<HubResult<Project>>;
  restore(repositoryId: string): Promise<HubResult<Project>>;
  confirm(resourceId: string): Promise<HubResult<ProjectResource>>;
}

const PAGE_SIZE = 50;

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function projectsApi(hub: Pick<Hub, 'request'>): ProjectsApi {
  return {
    ...groupsApi(hub),
    list: async (leftOut, after, browse) =>
      mapValue(await hub.request(PROJECTS, { leftOut, first: PAGE_SIZE, after, browse }), ({ projects }) => ({
        languages: projects.languages,
        projects: projects.edges.map((edge) => edge.node),
        next: projects.pageInfo.hasNextPage ? projects.pageInfo.endCursor : null,
      })),
    find: async (slug) => mapValue(await hub.request(PROJECT, { slug }), (value) => value.project ?? null),
    leaveOut: async (id) => mapValue(await hub.request(LEAVE_OUT, { id }), (value) => value.leaveOutRepository),
    restore: async (id) => mapValue(await hub.request(RESTORE, { id }), (value) => value.restoreRepository),
    confirm: async (id) => mapValue(await hub.request(CONFIRM, { id }), (value) => value.confirmProjectResource),
  };
}
