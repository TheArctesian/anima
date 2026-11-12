/** What the form edits of a device's session policy: the limit stays text so that "2.5" is reportable. */
export interface PolicyValues {
  /** Empty means no limit. */
  readonly sessionLimit: string;
}

/** The policy as the hub stores it (K1 `Device.sessionLimit`). */
export interface StoredPolicy {
  readonly sessionLimit: number | null;
}

/** The part of `CreateDeviceInput` and `UpdateDeviceInput` that carries the policy. */
export interface PolicyInput {
  sessionLimit?: number;
}

const MAX_SESSION_LIMIT = 100;

/** RA-036: the one sentence for a limit that is not a whole number from 1 to 100. */
export const LIMIT_SENTENCE = 'Use a whole number from 1 to 100.';

/** RA-036: a new device has no limit. */
export const DEFAULT_POLICY: PolicyValues = { sessionLimit: '' };

export function policyValues(stored: StoredPolicy): PolicyValues {
  return { sessionLimit: stored.sessionLimit === null ? '' : String(stored.sessionLimit) };
}

/** The limit typed, or null for none; undefined when the text is not a whole number. */
function typedLimit(text: string): number | null | undefined {
  const typed = text.trim();
  if (typed === '') return null;
  return /^\d{1,4}$/.test(typed) ? Number(typed) : undefined;
}

/**
 * RA-036: the sentence for a limit the hub would refuse, or null. A new device takes 1..100; an edit also takes 0,
 * which clears the limit.
 */
export function limitProblem(values: PolicyValues, creating: boolean): string | null {
  const limit = typedLimit(values.sessionLimit);
  if (limit === null) return null;
  if (limit === undefined || limit > MAX_SESSION_LIMIT) return LIMIT_SENTENCE;
  return creating && limit === 0 ? LIMIT_SENTENCE : null;
}

/** RA-036: a new device sends only what differs from the hub's default (no limit). */
export function createPolicyInput(values: PolicyValues): PolicyInput {
  const limit = typedLimit(values.sessionLimit);
  return limit === null || limit === undefined ? {} : { sessionLimit: limit };
}

/**
 * RA-036: only what differs from `stored`. A limit emptied or set to 0 is sent as 0, which clears a stored limit;
 * one that was already none sends nothing.
 */
export function updatePolicyInput(stored: PolicyValues, values: PolicyValues): PolicyInput {
  const before = typedLimit(stored.sessionLimit) ?? 0;
  const now = typedLimit(values.sessionLimit) ?? 0;
  return now === before ? {} : { sessionLimit: now };
}
