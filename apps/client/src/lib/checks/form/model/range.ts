import type { FieldError } from '$lib/hub/hub';

/** The largest value of a GraphQL `Int`. */
export const MAX_INT = 2_147_483_647;
export const MAX_PORT = 65_535;
/** The hub's limits for a check timeout, in milliseconds. */
export const TIMEOUT_MS = { min: 100, max: 60_000 } as const;

/** The number a validated text field holds. */
export function toNumber(text: string): number {
  return Number(text.trim());
}

/**
 * The hub's limits, mirrored so that the form refuses what the hub would. Each comes from:
 * the `checks_*_check` constraints in `crates/hub/migrations/0300_checks_schema.sql` (name,
 * interval, retries, retry interval, degraded threshold); `crates/hub/src/checks/config/rules.rs`
 * (URL, header, body, keyword and JSON query lengths, header count); `config/http.rs` (status
 * ranges, basic user name); `config.rs` (DNS `expected`); `checks/graphql/form.rs` (group children).
 */
export const LIMITS = {
  nameChars: 200,
  intervalSeconds: [10, 86_400],
  retries: [0, 10],
  retryIntervalSeconds: [5, 3600],
  degradedThresholdMs: [1, 600_000],
  urlBytes: 2048,
  bodyBytes: 64 * 1024,
  keywordBytes: 1024,
  jsonQueryBytes: 512,
  headerNameBytes: 128,
  headers: 20,
  statusRanges: 20,
  usernameBytes: 256,
  dnsExpectedBytes: 1024,
  groupChildren: 100,
} as const;

/** The length of `text` in UTF-8 bytes, which is how the hub measures its limits. */
export function byteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

/** An error on `field` when `text` is longer than `max` bytes. */
export function lengthError(field: string, text: string, max: number, what: string): FieldError[] {
  return byteLength(text) > max ? [{ field, message: `${what} can be at most ${String(max)} bytes long.` }] : [];
}

/** Whether `text` (trimmed) is a whole number from `min` to `max`. */
export function isIntegerIn(text: string, min: number, max: number = MAX_INT): boolean {
  const trimmed = text.trim();
  return /^\d{1,10}$/.test(trimmed) && Number(trimmed) >= min && Number(trimmed) <= max;
}

/** An error on `field` unless `text` is a whole number from `min` to `max`. */
export function integerError(field: string, text: string, [min, max]: readonly [number, number], message: string): FieldError[] {
  return isIntegerIn(text, min, max) ? [] : [{ field, message }];
}

export function timeoutError(text: string): FieldError[] {
  return integerError('timeoutMs', text, [TIMEOUT_MS.min, TIMEOUT_MS.max], `Enter a timeout from ${String(TIMEOUT_MS.min)} to ${String(TIMEOUT_MS.max)} ms.`);
}

export function portError(text: string): FieldError[] {
  if (text.trim() === '') return [{ field: 'port', message: 'Enter a port.' }];
  return integerError('port', text, [1, MAX_PORT], 'Enter a port from 1 to 65535.');
}
