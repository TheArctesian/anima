import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Project, ProjectRepository, ProjectResource } from './api';
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

/** A hub that answers the project calls from `start` and applies the changes as the hub does (PRO-002, PRO-004). */
export function fakeHub(start: readonly Project[] = FIXTURE_PROJECTS): FakeHub {
  let all = start;
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
        const edges = all.filter((project) => everyLeftOut(project) === wanted).map((node) => ({ node }));
        return ok({ projects: { edges, pageInfo: { hasNextPage: false, endCursor: null } } });
      }
      if (name === 'Project') return ok({ project: all.find((project) => project.slug === id) ?? null });
      hub.sent.push(`${name}:${id}`);
      if (hub.refuse !== null) return Promise.resolve({ ok: false, error: hub.refuse } as HubResult<TResult>);
      if (name === 'LeaveOutRepository') return ok({ leaveOutRepository: setLeftOut(id, true) });
      if (name === 'RestoreRepository') return ok({ restoreRepository: setLeftOut(id, false) });
      const confirmed = all.flatMap((project) => project.resources).find((each) => each.id === id) as ProjectResource;
      return ok({ confirmProjectResource: { ...confirmed, state: 'CONFIRMED' } });
    },
  };
  return hub;
}
