import type { HubError } from '$lib/hub/hub';
import type { BuildTarget } from './api';

/** What the "Release <version>" form holds (BLD-001). */
export interface ReleaseValues {
  readonly version: string;
  readonly commit: string;
  readonly targets: readonly BuildTarget[];
}

/** One message for each field that is wrong; a field that is fine has none. */
export interface ReleaseErrors {
  readonly version?: string;
  readonly commit?: string;
  readonly targets?: string;
}

/** The hub's own check: three numbers with dots between, each of 1 to 9 digits and no leading zero (BLD-002). */
const VERSION = /^(?:0|[1-9]\d{0,8})(?:\.(?:0|[1-9]\d{0,8})){2}$/;
/** A full commit: 40 lower-case hex digits. */
const COMMIT = /^[0-9a-f]{40}$/;

/** The mistakes the operator can see before asking the hub, with the hub's words (BLD-001, BLD-002). */
export function validateRelease(values: ReleaseValues): ReleaseErrors {
  return {
    ...(VERSION.test(values.version) ? {} : { version: 'A version is three numbers with dots between, such as 0.1.12.' }),
    ...(COMMIT.test(values.commit) ? {} : { commit: 'A commit is 40 lower-case hex digits.' }),
    ...(values.targets.length > 0 ? {} : { targets: 'Select at least one target.' }),
  };
}

/** `values` with the target added or removed, in the order the form lists targets. */
export function withTarget(values: readonly BuildTarget[], target: BuildTarget, selected: boolean, order: readonly BuildTarget[]): BuildTarget[] {
  const chosen = new Set(values);
  if (selected) chosen.add(target);
  else chosen.delete(target);
  return order.filter((known) => chosen.has(known));
}

/** The hub's refusal for the operator: the message of each rejected field, or the error's own sentence (BLD-002). */
export function refusalText(error: HubError): string {
  if (error.code === 'VALIDATION' && error.fields.length > 0) return error.fields.map((field) => field.message).join(' ');
  return error.message;
}
