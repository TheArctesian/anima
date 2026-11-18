import type { LogFilter } from '$lib/events/log';

/** The chips of the Logs page in the phone layout (mockup 6), each one filter; one is on at a time. */
export const CHIPS = [
  { id: 'all', label: 'All', dot: null },
  { id: 'errors', label: 'Errors', dot: 'down' },
  { id: 'warnings', label: 'Warnings', dot: 'degraded' },
  { id: 'info', label: 'Info', dot: null },
  { id: 'audit', label: 'Audit', dot: null },
] as const;

export type ChipId = (typeof CHIPS)[number]['id'];

/** The filter a chip sets; the time window and the device stay as they are. Audit is the access events (LOG-009). */
export function chipFilter(chip: ChipId, filter: LogFilter): LogFilter {
  const base = { ...filter, severities: [], sources: [] };
  if (chip === 'errors') return { ...base, severities: ['ERROR'] };
  if (chip === 'warnings') return { ...base, severities: ['WARNING'] };
  if (chip === 'info') return { ...base, severities: ['INFO'] };
  if (chip === 'audit') return { ...base, sources: ['ACCESS'] };
  return base;
}

/** The chip that shows a filter as on; "All" for a filter that no single chip sets. */
export function chipOf(filter: LogFilter): ChipId {
  const one = filter.severities.length === 1 ? filter.severities[0] : undefined;
  if (filter.sources.length === 1 && filter.sources[0] === 'ACCESS' && filter.severities.length === 0) return 'audit';
  if (filter.sources.length > 0) return 'all';
  if (one === 'ERROR') return 'errors';
  if (one === 'WARNING') return 'warnings';
  return one === 'INFO' ? 'info' : 'all';
}
