import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Project, ProjectBrowse, ProjectRepository, ProjectResource } from './api';
import type { ProjectGroup } from './groups/api';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';

// Test support: projects in the hub's shape and a hub that keeps them in memory.

export function repository(id: string, fullName: string, more: Partial<ProjectRepository> = {}): ProjectRepository {
  return { id, fullName, url: `https://github.com/${fullName}`, private: false, archived: false, fork: false, leftOut: false, gone: false, ...more };
}

export function resource(id: string, more: Partial<ProjectResource> = {}): ProjectResource {
  return { id, kind: 'CLOUDFLARE', state: 'FOUND_IN_REPOSITORY', evidence: [{ sign: 'WRANGLER_CONFIG', path: 'wrangler.toml' }], ...more };
}

export const FIXTURE_PROJECTS: readonly Project[] = [
  { id: 'p1', slug: 'acme-site', name: 'acme-site', repositories: [repository('r1', 'acme/acme-site')], resources: [resource('x1')] },
  { id: 'p2', slug: 'old-tool', name: 'old-tool', repositories: [repository('r2', 'acme/old-tool', { gone: true })], resources: [] },
  { id: 'p3', slug: 'notes', name: 'notes', repositories: [repository('r3', 'acme/notes', { archived: true })], resources: [] },
  { id: 'p4', slug: 'scratch', name: 'scratch', repositories: [repository('r4', 'acme/scratch', { leftOut: true })], resources: [] },
];

export interface FakeHub extends Pick<Hub, 'request'> {
  /** The operations sent, in order, as `Name:id`. */
  readonly sent: string[];
  /** What the next change answers instead of changing anything. */
  refuse: HubError | null;
}

const ok = <T>(value: unknown): Promise<HubResult<T>> => Promise.resolve({ ok: true, value } as HubResult<T>);

/** A hub that answers the project calls from `start` and applies the changes as the hub does (PRO-002, PRO-004, PRO-059 to PRO-063). */
export function fakeHub(start: readonly Project[] = FIXTURE_PROJECTS, startGroups: readonly ProjectGroup[] = []): FakeHub {
  let all = start;
  const groups = new GroupsHub(startGroups, () => all, (next) => (all = next));
  const everyLeftOut = (project: Project) => project.repositories.every((each) => each.leftOut);
  const setLeftOut = (id: string, leftOut: boolean): Project => {
    const changed = all.find((project) => project.repositories.some((each) => each.id === id)) as Project;
    const next = { ...changed, repositories: changed.repositories.map((each) => (each.id === id ? { ...each, leftOut } : each)) };
    all = all.map((project) => (project.id === next.id ? next : project));
    return next;
  };
  const hub: FakeHub = {
    sent: [],
    refuse: null,
    request: <TResult, TVariables extends Record<string, unknown>>(document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      const name = (document.definitions[0] as { name?: { value: string } }).name?.value ?? '';
      const id = String(variables['id'] ?? variables['slug'] ?? '');
      if (name === 'Projects') {
        const wanted = variables['leftOut'] === true;
        const browse = (variables['browse'] ?? {}) as ProjectBrowse;
        const edges = all
          .filter((project) => everyLeftOut(project) === wanted && groups.shows(project, browse))
          .map((node) => ({ node }));
        return ok({ projects: { edges, pageInfo: { hasNextPage: false, endCursor: null } } });
      }
      const reads: Record<string, () => unknown> = {
        Project: () => ({ project: all.find((project) => project.slug === id) ?? null }),
        ProjectGroups: () => ({ projectGroups: groups.list() }),
      };
      const read = reads[name];
      if (read !== undefined) return ok(read());
      hub.sent.push(`${name}:${id}`);
      if (hub.refuse !== null) return Promise.resolve({ ok: false, error: hub.refuse } as HubResult<TResult>);
      const grouped = groups.answer(name, variables);
      if (grouped !== undefined) return ok(grouped);
      if (name === 'LeaveOutRepository') return ok({ leaveOutRepository: setLeftOut(id, true) });
      if (name === 'RestoreRepository') return ok({ restoreRepository: setLeftOut(id, false) });
      const confirmed = all.flatMap((project) => project.resources).find((each) => each.id === id) as ProjectResource;
      return ok({ confirmProjectResource: { ...confirmed, state: 'CONFIRMED' } });
    },
  };
  return hub;
}

/** The group calls of the fake hub, which keep the group of each project in the projects as the hub does. */
class GroupsHub {
  #groups: ProjectGroup[];

  constructor(
    start: readonly ProjectGroup[],
    private readonly projects: () => readonly Project[],
    private readonly setProjects: (next: readonly Project[]) => void,
  ) {
    this.#groups = [...start];
  }

  shows(project: Project, browse: ProjectBrowse): boolean {
    if (browse.ungrouped === true) return project.group == null;
    return browse.group == null || project.group?.id === browse.group;
  }

  list(): ProjectGroup[] {
    const counted = (id: string) => this.projects().filter((project) => project.group?.id === id && project.repositories.some((each) => !each.leftOut)).length;
    return this.#groups.map((group) => ({ ...group, projectCount: counted(group.id) }));
  }

  /** The answer to a group change, or undefined for a call that is not one. */
  answer(name: string, variables: Record<string, unknown>): unknown {
    const ids = (variables['projects'] ?? []) as string[];
    if (name === 'CreateProjectGroup') {
      const group = { id: `g${String(this.#groups.length + 1)}`, name: String(variables['name']), projectCount: ids.length };
      this.#groups.push(group);
      this.#put(ids, group);
      return { createProjectGroup: group };
    }
    if (name === 'SetProjectsGroup') {
      this.#put(ids, this.#groups.find((group) => group.id === variables['group']) ?? null);
      return { setProjectsGroup: ids };
    }
    if (name === 'RenameProjectGroup') {
      this.#groups = this.#groups.map((group) => (group.id === variables['id'] ? { ...group, name: String(variables['name']) } : group));
      const renamed = this.#groups.find((group) => group.id === variables['id']);
      this.setProjects(this.projects().map((project) => (project.group?.id === renamed?.id ? { ...project, group: renamed } : project)));
      return { renameProjectGroup: renamed };
    }
    if (name === 'DeleteProjectGroup') {
      this.#groups = this.#groups.filter((group) => group.id !== variables['id']);
      this.setProjects(this.projects().map((project) => (project.group?.id === variables['id'] ? { ...project, group: null } : project)));
      return { deleteProjectGroup: variables['id'] };
    }
    return undefined;
  }

  #put(ids: readonly string[], group: ProjectGroup | null): void {
    const value = group === null ? null : { id: group.id, name: group.name };
    this.setProjects(this.projects().map((project) => (ids.includes(project.id) ? { ...project, group: value } : project)));
  }
}
