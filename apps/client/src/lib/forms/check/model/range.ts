import type { FieldError } from '$lib/hub/error';

/** The largest value of a GraphQL `Int`. */
export const MAX_INT = 2_147_483_647;
export const MAX_PORT = 65_535;
/** The hub's limits for a check timeout, in milliseconds. */
export const TIMEOUT_MS = { min: 100, max: 60_000 } as const;

/** Whether `text` (trimmed) is a whole number from `min` to `max`. */
export function isIntegerIn(text: string, min: number, max: number = MAX_INT): boolean {
  const trimmed = text.trim();
  return /^\d{1,10}$/.test(trimmed) && Number(trimmed) >= min && Number(trimmed) <= max;
}

/** An error on `field` unless `text` is a whole number from `min` to `max`. */
export function integerError(field: string, text: string, min: number, max: number, message: string): FieldError[] {
  return isIntegerIn(text, min, max) ? [] : [{ field, message }];
}

export function timeoutError(text: string): FieldError[] {
  return integerError('timeoutMs', text, TIMEOUT_MS.min, TIMEOUT_MS.max, `Enter a timeout from ${String(TIMEOUT_MS.min)} to ${String(TIMEOUT_MS.max)} ms.`);
}

export function portError(text: string): FieldError[] {
  if (text.trim() === '') return [{ field: 'port', message: 'Enter a port.' }];
  return integerError('port', text, 1, MAX_PORT, 'Enter a port from 1 to 65535.');
}
