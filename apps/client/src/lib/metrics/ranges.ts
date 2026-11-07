import type { MetricRange } from './api';

/** The metric ranges an operator can choose (MET-003): the hub's enum, its URL word, its label and how a chart's text alternative says it. */
export const RANGES = [
  { range: 'HOUR', param: 'hour', label: '1 hour', phrase: 'last hour' },
  { range: 'DAY', param: 'day', label: '24 hours', phrase: 'last 24 hours' },
  { range: 'WEEK', param: 'week', label: '7 days', phrase: 'last 7 days' },
] as const satisfies readonly { range: MetricRange; param: string; label: string; phrase: string }[];
