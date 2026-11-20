import type { Project } from '../api';
import type { ProjectGroup } from './api';

/** How the Projects list puts its projects together: by their state or by their group (PRO-061). */
export type ShowBy = 'status' | 'group';

/** One part of the list shown by group: a group, or the projects in no group (`id` null). */
export interface GroupSection {
  readonly id: string | null;
  readonly title: string;
  readonly projects: readonly Project[];
}

export const UNGROUPED = 'Ungrouped';

/**
 * The groups in name order, then "Ungrouped" (PRO-061). A group with no project in the list is shown only when
 * `withEmpty`, so that a filter does not fill the page with empty heads; "Ungrouped" is shown only when it holds one.
 */
export function sectionsByGroup(projects: readonly Project[], groups: readonly ProjectGroup[], withEmpty: boolean): GroupSection[] {
  const named = groups
    .toSorted((a, b) => a.name.localeCompare(b.name))
    .map((group) => ({ id: group.id, title: group.name, projects: projects.filter((project) => project.group?.id === group.id) }))
    .filter((section) => withEmpty || section.projects.length > 0);
  const known = new Set(groups.map((group) => group.id));
  const rest = projects.filter((project) => project.group == null || !known.has(project.group.id));
  return rest.length > 0 ? [...named, { id: null, title: UNGROUPED, projects: rest }] : named;
}

/** The projects with the group set, or removed with null, for the IDs given. */
export function withGroup(projects: readonly Project[], ids: readonly string[], group: Project['group']): Project[] {
  return projects.map((project) => (ids.includes(project.id) ? { ...project, group: group ?? null } : project));
}

/** The projects with the group renamed, or removed when `name` is null (the group was deleted). */
export function withGroupNamed(projects: readonly Project[], id: string, name: string | null): Project[] {
  return projects.map((project) => (project.group?.id === id ? { ...project, group: name === null ? null : { id, name } } : project));
}

const SHOW_BY = 'anima.projects.showBy';
const CLOSED = 'anima.projects.closedGroups';

/** The view this computer last showed; "status" when none was kept or the storage refuses. */
export function storedShowBy(storage: Storage): ShowBy {
  try {
    return storage.getItem(SHOW_BY) === 'group' ? 'group' : 'status';
  } catch {
    return 'status';
  }
}

/** The group heads that are closed on this computer ("" is "Ungrouped"). */
export function storedClosed(storage: Storage): string[] {
  try {
    const kept: unknown = JSON.parse(storage.getItem(CLOSED) ?? '[]');
    return Array.isArray(kept) ? kept.filter((each): each is string => typeof each === 'string') : [];
  } catch {
    return [];
  }
}

/** Keeps the view on this computer; a storage that refuses keeps nothing, and the view still works. */
export function storeView(storage: Storage, showBy: ShowBy, closed: readonly string[]): void {
  try {
    storage.setItem(SHOW_BY, showBy);
    storage.setItem(CLOSED, JSON.stringify(closed));
  } catch {
    // The choice holds for this visit only.
  }
}
