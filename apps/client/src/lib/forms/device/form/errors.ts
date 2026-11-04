import type { FieldError } from '$lib/hub/error';
import { FORM_FIELD } from '../field-errors';
import type { TestOutcome } from '../connection-test';

/** The fields that have an input, in the order they appear on screen. */
const FIELD_ORDER = ['name', 'host', 'siteId', 'ssh.port', 'ssh.username', 'credential.privateKey.key'] as const;

/** The first error of `field`, or undefined. */
export function errorFor(errors: readonly FieldError[], field: string): FieldError | undefined {
  return errors.find((error) => error.field === field);
}

/** `errors` with those of `field` replaced by `next`; used when one input is validated alone (D-3, blur). */
export function replaceField(errors: readonly FieldError[], field: string, next: readonly FieldError[]): FieldError[] {
  return [...errors.filter((error) => error.field !== field), ...next.filter((error) => error.field === field)];
}

/** The errors that belong to no input (shown in the form-level alert). */
export function formErrors(errors: readonly FieldError[]): FieldError[] {
  return errors.filter((error) => error.field === FORM_FIELD || !FIELD_ORDER.some((field) => field === error.field));
}

/** The field to focus first: the earliest on screen that has an error. */
export function firstInvalidField(errors: readonly FieldError[]): string | undefined {
  return FIELD_ORDER.find((field) => errors.some((error) => error.field === field));
}

/** The DOM id of the input for a field path. */
export function inputId(field: string): string {
  return `device-${field.replaceAll('.', '-')}`;
}

/** The DOM id of the error text for a field path. */
export function errorId(field: string): string {
  return `${inputId(field)}-error`;
}

const CHANGED_KEY_WARNING = 'The host key changed. Do not trust this device until you check it.';

/** DEV-013, SEC-007: the sentence for a test outcome. */
export function outcomeText(outcome: TestOutcome): string {
  switch (outcome.kind) {
    case 'ok':
      return 'The device answered and accepted the credential.';
    case 'unreachable':
    case 'authFailed':
      return outcome.message;
    case 'hostKeyNew':
      return 'The device has a new host key. Check the fingerprint before you trust it.';
    case 'hostKeyChanged':
    case 'hostKeyChangedUnknown':
      return CHANGED_KEY_WARNING;
  }
}

/** Whether the outcome is one the operator must not ignore. */
export function isWarning(outcome: TestOutcome): boolean {
  return outcome.kind === 'hostKeyChanged' || outcome.kind === 'hostKeyChangedUnknown';
}
