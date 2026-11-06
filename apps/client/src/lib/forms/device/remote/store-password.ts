import type { HubError, HubResult } from '$lib/hub/error';
import type { PasswordCredentialInput } from '$lib/hub/generated/graphql';

const NOT_STORED: HubError = { code: 'INTERNAL', message: 'The hub cannot store this password yet. Enter it again later.' };

/** The call that stores an RDP or VNC password; absent until the devices client has it (H-1). */
export type PasswordStore = ((deviceId: string, credential: PasswordCredentialInput) => Promise<HubResult<unknown>>) | undefined;

/** Stores a typed password; with no call to store it the result is an error, so the caller never drops the password. */
export async function storePassword(
  store: PasswordStore,
  deviceId: string,
  credential: PasswordCredentialInput,
): Promise<HubResult<unknown>> {
  return store === undefined ? { ok: false, error: NOT_STORED } : store(deviceId, credential);
}
