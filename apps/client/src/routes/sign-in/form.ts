import { TOKEN_UNREADABLE, type FieldError, type HubError } from '$lib/hub/hub';

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

const REFUSED_DESKTOP = 'The hub refused the request. Check the VPN client and the hub address.';

/**
 * A fixed sentence for each way a sign-in can fail. The hub's own text is never shown here: it is the hub's
 * wording, not ours, and sign-in input must not come back in the page (LOG-007). Field messages are the exception, kept beside their field.
 * A browser has no hub address to check (CLI-002): its FORBIDDEN is a network the hub does not permit (NET-002) or a
 * page opened at an address other than the hub's own, whose Origin the hub refuses (K7).
 */
export function failureMessage(error: HubError, desktop: boolean): string {
  switch (error.code) {
    case 'UNAUTHENTICATED':
      // The request never left: the key store could not be read, which is not a wrong password.
      if (error === TOKEN_UNREADABLE) return 'The key store could not be read, so Anima cannot sign you in.';
      return 'The user name, password or code is wrong.';
    case 'VALIDATION':
      return 'Check the marked fields.';
    case 'unreachable':
      return 'The hub did not answer. Check the VPN client.';
    case 'FORBIDDEN':
      return desktop ? REFUSED_DESKTOP : 'The hub refused sign-in from here. Check the VPN client and the page address.';
    case 'malformed':
      return desktop ? REFUSED_DESKTOP : 'The hub refused the request. Check the VPN client.';
    case 'UNAVAILABLE':
      return 'The hub cannot reach its database. Try again shortly.';
    default:
      return 'The hub could not sign you in. Try again.';
  }
}

const FIELD_ORDER: readonly LoginField[] = ['username', 'password', 'totpCode'];

/** The first field in page order that has a message, so focus can go there after a failed submit. */
export function firstInvalid(fields: FieldMessages): LoginField | undefined {
  return FIELD_ORDER.find((field) => fields[field] !== undefined);
}

/** The messages after `field` was edited to `value`: that field's message goes once its value passes the client rules. */
export function afterEdit(fields: FieldMessages, field: LoginField, value: string): FieldMessages {
  const probe: LoginForm = { username: 'x', password: 'x', totpCode: '', [field]: value };
  if (fields[field] === undefined || checkLogin(probe)[field] !== undefined) return fields;
  return { ...fields, [field]: undefined };
}
