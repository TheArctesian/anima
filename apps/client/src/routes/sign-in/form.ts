import type { FieldError, HubError } from '$lib/hub/hub';

/** The three fields of the sign-in form. */
export type LoginField = 'username' | 'password' | 'totpCode';

export interface LoginForm {
  readonly username: string;
  readonly password: string;
  readonly totpCode: string;
}

/** The message to show beside each field that has one. */
export type FieldMessages = Partial<Record<LoginField, string>>;

const TOTP_SHAPE = /^\d{6}$/;

/** Problems the client sees before sending anything. An empty code is allowed: an account may have no second factor (OD-2). */
export function checkLogin(form: LoginForm): FieldMessages {
  const problems: FieldMessages = {};
  if (form.username.trim() === '') problems.username = 'Type your user name.';
  if (form.password === '') problems.password = 'Type your password.';
  if (form.totpCode !== '' && !TOTP_SHAPE.test(form.totpCode)) problems.totpCode = 'The code is six digits.';
  return problems;
}

/** The hub's rejected fields that the form has. The hub names the input fields as the schema does (`username`, `password`, `totpCode`). */
export function fieldMessages(fields: readonly FieldError[]): FieldMessages {
  const messages: FieldMessages = {};
  for (const { field, message } of fields) {
    if (field === 'username' || field === 'password' || field === 'totpCode') messages[field] = message;
  }
  return messages;
}

/**
 * A fixed sentence for each way a sign-in can fail. The hub's own text is never shown here: it is the hub's
 * wording, not ours, and sign-in input must not come back in the page (LOG-007). Field messages are the exception, kept beside their field.
 */
export function failureMessage(error: HubError): string {
  switch (error.code) {
    case 'UNAUTHENTICATED':
      return 'The user name, password or code is wrong.';
    case 'VALIDATION':
      return 'Check the marked fields.';
    case 'unreachable':
      return 'The hub did not answer. Check the VPN client.';
    case 'FORBIDDEN':
    case 'malformed':
      return 'The hub refused the request. Check the VPN client and the hub address.';
    case 'UNAVAILABLE':
      return 'The hub cannot reach its database. Try again shortly.';
    default:
      return 'The hub could not sign you in. Try again.';
  }
}
