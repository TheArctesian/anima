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

/** `values` with `value` added, or removed when it is there already. */
export function toggled<T>(values: readonly T[], value: T): T[] {
  return values.includes(value) ? values.filter((known) => known !== value) : [...values, value];
}

/** Whether the query narrows the list (view and sort do not). */
export function isFiltered(query: ListQuery): boolean {
  return query.name !== '' || query.statuses.length > 0 || query.tags.length > 0;
}

/** The query with no filter; the view and the sort stay. */
export function withoutFilters(query: ListQuery): ListQuery {
  return { ...query, name: '', statuses: [], tags: [] };
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
