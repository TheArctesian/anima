import type { DeviceFilter, DeviceSort, DevicesQueryVariables, Status } from '$lib/hub/generated/graphql';

/** The device page state that lives in the URL, so Back and Forward restore the list (D-4). */
export interface ListQuery {
  readonly view: 'grid' | 'table';
  readonly name: string;
  readonly statuses: readonly Status[];
  readonly tags: readonly string[];
  /** A site ID, or null for every site (VPN-002). */
  readonly site: string | null;
  readonly sort: DeviceSort;
}

/** The first page holds the K1 maximum, so 200 devices render from one answer (DEV-001, API-006). */
export const PAGE_SIZE = 200;

export const DEFAULT_QUERY: ListQuery = { view: 'grid', name: '', statuses: [], tags: [], site: null, sort: 'NAME' };

const STATUSES: readonly Status[] = ['UP', 'DOWN', 'DEGRADED', 'MAINTENANCE', 'UNKNOWN'];
const SORTS: readonly DeviceSort[] = ['NAME', 'STATUS', 'UPTIME'];

/** The values of `allowed` that appear in `values`, in the order of `allowed`; anything else is dropped. */
function known<T extends string>(values: readonly string[], allowed: readonly T[]): T[] {
  return allowed.filter((value) => values.includes(value));
}

/** The query that `params` describe; the URL is typed by hand, so unknown values fall back to the defaults. */
export function parseListQuery(params: URLSearchParams): ListQuery {
  return {
    view: params.get('view') === 'table' ? 'table' : DEFAULT_QUERY.view,
    name: params.get('q') ?? DEFAULT_QUERY.name,
    statuses: known(params.getAll('status'), STATUSES),
    tags: [...new Set(params.getAll('tag').filter((tag) => tag !== ''))],
    site: params.get('site') || null,
    sort: known([params.get('sort') ?? ''], SORTS)[0] ?? DEFAULT_QUERY.sort,
  };
}

/** The search parameters for `query`: defaults are left out and the order is fixed (view, q, status, tag, site, sort). */
export function writeListQuery(query: ListQuery): URLSearchParams {
  const params = new URLSearchParams();
  if (query.view !== DEFAULT_QUERY.view) params.set('view', query.view);
  if (query.name !== '') params.set('q', query.name);
  for (const status of query.statuses) params.append('status', status);
  for (const tag of query.tags) params.append('tag', tag);
  if (query.site !== null) params.set('site', query.site);
  if (query.sort !== DEFAULT_QUERY.sort) params.set('sort', query.sort);
  return params;
}

/** The hub filter for `query`, with empty parts left out; none at all when nothing is filtered. */
function filterOf(query: ListQuery): DeviceFilter | null {
  const filter: DeviceFilter = {
    ...(query.name === '' ? {} : { name: query.name }),
    ...(query.statuses.length === 0 ? {} : { status: [...query.statuses] }),
    ...(query.tags.length === 0 ? {} : { tags: [...query.tags] }),
    ...(query.site === null ? {} : { siteId: query.site }),
  };
  return Object.keys(filter).length === 0 ? null : filter;
}

/** The `Devices` variables for `query`; `after` is the cursor of the page to load next. */
export function listVariables(query: ListQuery, after?: string | null): DevicesQueryVariables {
  const filter = filterOf(query);
  return {
    ...(filter === null ? {} : { filter }),
    sort: query.sort,
    first: PAGE_SIZE,
    ...(after == null ? {} : { after }),
  };
}
