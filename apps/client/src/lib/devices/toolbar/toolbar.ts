import type { DeviceSort, Status } from '$lib/hub/generated/graphql';
import { statusLabel } from '../card/card';
import type { ListQuery } from '../query';

/** The name filter sends its request this long after the last keystroke (D-4). It is not an animation timer (MOT-042). */
export const NAME_DEBOUNCE_MS = 200;

export interface MenuOption {
  readonly value: string;
  readonly label: string;
}

const STATUS_ORDER: readonly Status[] = ['UP', 'DOWN', 'DEGRADED', 'MAINTENANCE', 'UNKNOWN'];

export const STATUS_OPTIONS: readonly MenuOption[] = STATUS_ORDER.map((value) => ({
  value,
  label: statusLabel(value),
}));

export const SORT_OPTIONS: readonly (MenuOption & { readonly value: DeviceSort })[] = [
  { value: 'NAME', label: 'Name' },
  { value: 'STATUS', label: 'Status' },
  { value: 'UPTIME', label: 'Uptime' },
];

/** The value of the "All sites" choice: no site filter (VPN-002). */
export const ALL_SITES = '';

/** The Site menu: "All sites" first, then each site by name (VPN-002). */
export function siteOptions(sites: readonly { id: string; name: string }[]): readonly MenuOption[] {
  return [{ value: ALL_SITES, label: 'All sites' }, ...sites.map((site) => ({ value: site.id, label: site.name }))];
}

/** The Site button's text: "Site" for all sites, "Site: Lab" for one (VPN-002). */
export function siteButtonLabel(options: readonly MenuOption[], site: string | null): string {
  const chosen = site === null ? undefined : options.find((option) => option.value === site);
  return chosen === undefined ? 'Site' : `Site: ${chosen.label}`;
}

/** `values` with `value` added, or removed when it is there already. */
export function toggled<T>(values: readonly T[], value: T): T[] {
  return values.includes(value) ? values.filter((known) => known !== value) : [...values, value];
}

/** Whether the query narrows the list (view and sort do not). */
export function isFiltered(query: ListQuery): boolean {
  return query.name !== '' || query.statuses.length > 0 || query.tags.length > 0 || query.site !== null;
}

/** The query with no filter; the view and the sort stay. */
export function withoutFilters(query: ListQuery): ListQuery {
  return { ...query, name: '', statuses: [], tags: [], site: null };
}

/** A button label that counts the chosen values: "Status" or "Status (2)". */
export function menuLabel(label: string, chosen: number): string {
  return chosen === 0 ? label : `${label} (${String(chosen)})`;
}

export interface Debounced {
  (): void;
  cancel(): void;
}

/** Calls `run` once `ms` after the last call. */
export function debounced(run: () => void, ms: number): Debounced {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const call = () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      run();
    }, ms);
  };
  call.cancel = () => {
    clearTimeout(timer);
    timer = undefined;
  };
  return call;
}

/**
 * How far to move a menu panel that starts at `left` and is `width` wide so it ends inside a viewport `viewport` wide,
 * `margin` from the edge. The left edge wins when the panel is wider than the viewport.
 */
export function shiftInsideViewport(left: number, width: number, viewport: number, margin: number): number {
  const toRight = Math.min(0, viewport - margin - (left + width));
  return Math.max(toRight, margin - left);
}
