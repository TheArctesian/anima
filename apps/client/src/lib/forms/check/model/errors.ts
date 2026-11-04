import type { FieldError } from '$lib/hub/error';

/** The first message for `field`, or undefined. */
export function messageFor(errors: readonly FieldError[], field: string): string | undefined {
  return errors.find((error) => error.field === field)?.message;
}

/**
 * The errors that no input shows: the form-level ones and the hub's field names this form has no
 * input for. They are listed at the top of the form so that none is lost.
 */
export function unplaced(errors: readonly FieldError[], shown: ReadonlySet<string>): FieldError[] {
  return errors.filter((error) => !shown.has(error.field) && !/^(headers|acceptedStatus)\.\d+/.test(error.field));
}
