import type { AccessApi } from '$lib/access/api';
import { storeHubAddress } from '$lib/desktop/address';
import { AccessTokenError } from '$lib/desktop/token';
import type { HubAddress, HubError } from '$lib/hub/hub';
import { checkLogin, failureMessage, fieldMessages, type FieldMessages, type LoginForm } from './form';

/** What sign-in needs; every part is injected so that tests need no desktop shell (CLI-003). */
export interface SignIn {
  readonly api: Pick<AccessApi, 'login' | 'signInBrowser'>;
  readonly address: HubAddress;
  /**
   * Whether the key store exists. The desktop app signs in with `login` and keeps the token there; a browser signs in
   * with `signInBrowser`, and the hub keeps the token in the `HttpOnly` cookie (CLI-003, D-3, E-4).
   */
  readonly desktop: boolean;
  readonly writeToken: (address: HubAddress, token: string) => Promise<void>;
}

export type SignInOutcome =
  | { readonly signedIn: true }
  | { readonly signedIn: false; readonly message: string; readonly fields: FieldMessages };

const NOT_KEPT_MESSAGE = 'Signed in, but the access token could not be kept in the key store.';

function refused(message: string, fields: FieldMessages = {}): SignInOutcome {
  return { signedIn: false, message, fields };
}

function failed(error: HubError, desktop: boolean): SignInOutcome {
  const fields = error.code === 'VALIDATION' ? fieldMessages(error.fields) : {};
  return refused(failureMessage(error, desktop), fields);
}

/**
 * Signs in with the form. On desktop the token goes from the hub's answer to the key store inside this function and
 * nowhere else; in a browser no token reaches script at all (CLI-003, LOG-007).
 */
export async function signIn({ api, address, desktop, writeToken }: SignIn, form: LoginForm): Promise<SignInOutcome> {
  const problems = checkLogin(form);
  if (Object.keys(problems).length > 0) return refused('Check the marked fields.', problems);
  const totpCode = form.totpCode === '' ? {} : { totpCode: form.totpCode };
  const input = { username: form.username.trim(), password: form.password, ...totpCode };
  if (!desktop) {
    const signedIn = await api.signInBrowser(input);
    return signedIn.ok ? { signedIn: true } : failed(signedIn.error, desktop);
  }
  const result = await api.login(input);
  if (!result.ok) return failed(result.error, desktop);
  try {
    await writeToken(address, result.value);
  } catch (reason) {
    // The key store's error is fixed text chosen by the client; anything else gets the generic sentence.
    return refused(reason instanceof AccessTokenError ? reason.message : NOT_KEPT_MESSAGE);
  }
  return { signedIn: true };
}

/** Save the hub address the operator typed and reload so the whole client uses it (CLI-002, NET-005). Returns the refusal to show, or `null`. */
export function saveHubAddress(storage: Storage, text: string, reload: () => void): string | null {
  const stored = storeHubAddress(storage, text);
  if (!stored.ok) return stored.error.message;
  reload();
  return null;
}
