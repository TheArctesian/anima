import type { AccessApi } from '$lib/access/api';
import { storeHubAddress } from '$lib/desktop/address';
import { AccessTokenError } from '$lib/desktop/token';
import type { HubAddress } from '$lib/hub/hub';
import { checkLogin, failureMessage, fieldMessages, type FieldMessages, type LoginForm } from './form';

/** What sign-in needs; every part is injected so that tests need no desktop shell (CLI-003). */
export interface SignIn {
  readonly api: Pick<AccessApi, 'login'>;
  readonly address: HubAddress;
  readonly writeToken: (address: HubAddress, token: string) => Promise<void>;
}

export type SignInOutcome =
  | { readonly signedIn: true }
  | { readonly signedIn: false; readonly message: string; readonly fields: FieldMessages };

const NOT_KEPT_MESSAGE = 'Signed in, but the access token could not be kept in the key store.';

function refused(message: string, fields: FieldMessages = {}): SignInOutcome {
  return { signedIn: false, message, fields };
}

/** The token goes from the hub's answer to the key store inside this function and nowhere else (CLI-003, LOG-007). */
export async function signIn({ api, address, writeToken }: SignIn, form: LoginForm): Promise<SignInOutcome> {
  const problems = checkLogin(form);
  if (Object.keys(problems).length > 0) return refused('Check the marked fields.', problems);
  const totpCode = form.totpCode === '' ? {} : { totpCode: form.totpCode };
  const result = await api.login({ username: form.username.trim(), password: form.password, ...totpCode });
  if (!result.ok) {
    const fields = result.error.code === 'VALIDATION' ? fieldMessages(result.error.fields) : {};
    return refused(failureMessage(result.error), fields);
  }
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
