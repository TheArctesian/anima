import type { IconName } from '$lib/icons/icons';
import type { Project, ResourceKind } from './api';
import { evidenceWords, resourceKindWords } from './words';

/** The groups of the Projects page, problems first (UR-7): what needs the operator, what runs, what runs nowhere yet. */
export type TileGroup = 'look' | 'deployed' | 'undeployed';

export const GROUP_TITLES: Readonly<Record<TileGroup, string>> = {
  look: 'Needs a look',
  deployed: 'Deployed',
  undeployed: 'Not deployed',
};

export const GROUP_ORDER: readonly TileGroup[] = ['look', 'deployed', 'undeployed'];

/** The mark of each place a project runs. */
export const KIND_ICONS: Readonly<Record<ResourceKind, IconName>> = {
  CLOUDFLARE: 'simple-icons:cloudflare',
  VERCEL: 'simple-icons:vercel',
  SERVER: 'carbon:bare-metal-server',
  DESKTOP_APPLICATION: 'carbon:screen',
};

/** Why a project needs a look, or `null` when it does not: a repository gone from GitHub, then a place not confirmed (PRO-004). */
export function lookReason(project: Pick<Project, 'repositories' | 'resources'>): string | null {
  if (project.repositories.some((repository) => repository.gone && !repository.leftOut)) {
    return 'A repository is no longer on GitHub. Leave it out.';
  }
  const proposed = project.resources.filter((resource) => resource.state === 'FOUND_IN_REPOSITORY');
  const [only] = proposed;
  if (only === undefined) return null;
  if (proposed.length > 1) return `${String(proposed.length)} places found in the repository, not confirmed.`;
  return `${resourceKindWords(only.kind)} found in the repository (${only.evidence.map(evidenceWords).join(', ')}), not confirmed.`;
}

export function groupOf(project: Pick<Project, 'repositories' | 'resources'>): TileGroup {
  if (lookReason(project) !== null) return 'look';
  return project.resources.some((resource) => resource.state === 'CONFIRMED') ? 'deployed' : 'undeployed';
}
