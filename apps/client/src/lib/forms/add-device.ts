import type { HubError, HubResult } from '$lib/hub/error';
import type { CreateDeviceInput, SshCredentialInput } from './device/hub-types';
import { createInput, credentialInput } from './device/input';
import type { DeviceFormValues } from './device/values';

/** The two hub calls adding a device takes; the route supplies them from the devices client (D-1). */
export interface AddDeviceApi {
  createDevice: (input: CreateDeviceInput) => Promise<HubResult<{ readonly id: string }>>;
  setSshCredential: (deviceId: string, credential: SshCredentialInput) => Promise<HubResult<unknown>>;
}

export interface AddedDevice {
  readonly id: string;
  /** Set when the device exists but its credential was not stored; the operator must enter it again. */
  readonly credentialError: HubError | null;
}

/**
 * D-6: creates the device with its default check, then stores the typed credential. A failed credential
 * does not undo the device: the result is still `ok` with the ID and the error, so the route can open
 * the device and say what is missing.
 */
export async function addDevice(
  values: DeviceFormValues,
  siteId: string,
  api: AddDeviceApi,
): Promise<HubResult<AddedDevice>> {
  const created = await api.createDevice(createInput(values, siteId));
  if (!created.ok) return created;
  const { id } = created.value;
  const credential = credentialInput(values);
  if (credential === null) return { ok: true, value: { id, credentialError: null } };
  const stored = await api.setSshCredential(id, credential);
  return { ok: true, value: { id, credentialError: stored.ok ? null : stored.error } };
}
