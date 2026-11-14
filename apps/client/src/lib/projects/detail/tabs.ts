import type { IconName } from '$lib/icons/icons';

/** The project page's tabs, in order (PRJ-030, UR-7): HQ's GitHub work laid out as Anima's detail pages. */
export const PROJECT_TABS = [
  { id: 'overview', label: 'Overview', icon: 'carbon:dashboard' },
  { id: 'work', label: 'Work', icon: 'carbon:task-complete' },
  { id: 'code', label: 'Code', icon: 'carbon:code' },
  { id: 'history', label: 'History', icon: 'carbon:recently-viewed' },
] as const satisfies readonly { readonly id: string; readonly label: string; readonly icon: IconName }[];

export type ProjectTab = (typeof PROJECT_TABS)[number]['id'];

/**
 * The tabs whose function the hub serves today. A tab shows only once it is built (PRJ-030): Work, Code and History
 * join when the hub reads pull requests, issues, runs, commits and files.
 */
export const BUILT_TABS: readonly ProjectTab[] = ['overview'];

/** The tab a `?tab=` word names, when that tab is built; Overview otherwise. */
export function readTab(word: string | null, built: readonly ProjectTab[] = BUILT_TABS): ProjectTab {
  return built.find((tab) => tab === word) ?? 'overview';
}
