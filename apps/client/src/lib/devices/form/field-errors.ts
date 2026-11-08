import type { FieldError, HubError } from '$lib/hub/hub';

/** The field path of an error that belongs to no input. */
export const FORM_FIELD = '';

/** D-2: hub `VALIDATION` fields (camelCase paths) in the client's shape; any other code is one form-level error. */
export function fieldErrorsFrom(error: HubError): FieldError[] {
  if (error.code === 'VALIDATION' && error.fields.length > 0) return [...error.fields];
  return [{ field: FORM_FIELD, message: error.message }];
}
