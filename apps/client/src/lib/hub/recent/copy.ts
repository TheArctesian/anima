import type { DocumentNode } from 'graphql';
import type { HubAddress } from '../address';

/**
 * The queries whose last answer is kept on this computer (FND-007): devices, checks, recent metrics and the event
 * log, with the lists that their screens need. A query is added here only when its answer holds no credential.
 */
export const COPIED_QUERIES: readonly string[] = [
  'Devices',
  'Device',
  'Tags',
  'Sites',
  'DeviceChecks',
  'CheckHeartbeats',
  'DeviceMetrics',
  'Events',
];

/** One kept answer: the JSON text of the value and when the hub gave it. */
export interface Copy {
  readonly key: string;
  /** Milliseconds since the epoch. */
  readonly at: number;
  readonly text: string;
}

/** Where copies are kept. Every call may reject: the caller treats that as no copy. */
export interface CopyStore {
  read(key: string): Promise<Copy | undefined>;
  /** Keeps `copy` in place of any copy with the same key, then drops the oldest copies above `most`. */
  write(copy: Copy, most: number): Promise<void>;
  remove(key: string): Promise<void>;
  /** Removes every copy, for sign-out. */
  clear(): Promise<void>;
}

/** The size of the copy on this computer (FND-007: the size must fit the device). */
export interface CopyLimits {
  /** The most answers kept; the oldest go first. */
  readonly most: number;
  /** An answer with a longer JSON text is not kept. */
  readonly longest: number;
  /** A copy older than this is not shown. */
  readonly oldestMs: number;
}

const DAY_MS = 24 * 60 * 60 * 1000;

/** A computer: 100 answers of at most 1 MB of text each, shown for 30 days. */
export const DESKTOP_LIMITS: CopyLimits = { most: 100, longest: 1_000_000, oldestMs: 30 * DAY_MS };

/** The name of `document` when it is one copied query; `null` for a mutation, a subscription or any other query. */
export function copiedQuery(document: DocumentNode): string | null {
  const [definition] = document.definitions;
  if (definition?.kind !== 'OperationDefinition' || definition.operation !== 'query') return null;
  const name = definition.name?.value;
  return name !== undefined && COPIED_QUERIES.includes(name) ? name : null;
}

/** The key of one answer: the hub, the query and its variables, so that a copy never answers for a different hub. */
export function copyKey(address: HubAddress, query: string, variables: unknown): string {
  return `${address.http}\n${query}\n${JSON.stringify(variables)}`;
}

/** Whether a copy made at `at` may still be shown at `now`. */
export function isShown(at: number, now: number, limits: CopyLimits): boolean {
  return now - at <= limits.oldestMs;
}
