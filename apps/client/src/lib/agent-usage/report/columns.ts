import type { Column } from '$lib/tables/table';
import type { UsageRow } from '../api';
import { tokens, whole } from './units';
import { costText, sourceName } from './words';

// The columns of the table: one key column for each dimension the rows are summed by, then the counts (as `anima agents usage`).

/** What the hub can sum the use by. */
type AgentUsageDimension = 'DAY' | 'DEVICE' | 'SOURCE' | 'MODEL' | 'REPOSITORY';

/** A key as the row names it; a model or repository that the use has none of reads as words, never as an empty cell. */
const KEYS: Readonly<Record<AgentUsageDimension, Column<UsageRow>>> = {
  DAY: { key: 'day', header: 'Day', min: 7, max: 8, value: (row) => row.day ?? '' },
  DEVICE: { key: 'device', header: 'Device', min: 6, flex: true, value: (row) => row.device?.name ?? '' },
  SOURCE: { key: 'source', header: 'Program', min: 6, max: 8, value: (row) => (row.source == null ? '' : sourceName(row.source)) },
  MODEL: { key: 'model', header: 'Model', min: 7, flex: true, value: (row) => row.model ?? 'No model' },
  REPOSITORY: { key: 'repository', header: 'Repository', min: 8, flex: true, value: (row) => row.repository ?? 'No repository' },
};

function count(key: string, header: string, value: (row: UsageRow) => string, min = 4.5): Column<UsageRow> {
  return { key, header, kind: 'number', min, max: min + 1, value };
}

const COUNTS: readonly Column<UsageRow>[] = [
  count('sessions', 'Sessions', (row) => String(row.sessions), 5),
  count('prompts', 'Prompts', (row) => whole(row.prompts), 5),
  count('toolCalls', 'Tool calls', (row) => whole(row.toolCalls), 5),
  count('input', 'Input', (row) => tokens(row.inputTokens)),
  count('output', 'Output', (row) => tokens(row.outputTokens)),
  count('cacheRead', 'Cache read', (row) => tokens(row.cacheReadTokens), 5),
  count('cacheWrite', 'Cache write', (row) => tokens(row.cacheWriteTokens), 5),
  count('cost', 'Est. cost (USD)', costText, 6),
];

/** The key columns for `group` in table order (day, device, program, model, repository), then the counts. */
export function usageColumns(group: readonly AgentUsageDimension[]): readonly Column<UsageRow>[] {
  const keys = (Object.keys(KEYS) as AgentUsageDimension[]).filter((dimension) => group.includes(dimension));
  return [...keys.map((dimension) => KEYS[dimension]), ...COUNTS];
}

/** A key for the table's rows: the keys of the group, which are unique among the hub's rows. */
export function usageRowKey(row: UsageRow): string {
  return [row.day, row.device?.id, row.source, row.model, row.repository].map((part) => part ?? '').join('|');
}
