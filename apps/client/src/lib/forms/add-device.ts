import type { HubError, HubResult } from '$lib/hub/error';
import type { CreateDeviceInput, SshCredentialInput } from '$lib/hub/generated/graphql';
import { createInput, credentialInput } from './device/input';
import { siteFieldErrors, type SitePlacement } from './device/site';
import type { DeviceFormValues } from './device/values';

/** The hub calls adding a device takes; the route supplies them from the devices client (D-1). */
export interface AddDeviceApi {
  sites: () => Promise<HubResult<readonly { readonly id: string; readonly subnet: string }[]>>;
  createSite: (name: string, subnet: string) => Promise<HubResult<{ readonly id: string }>>;
  createDevice: (input: CreateDeviceInput) => Promise<HubResult<{ readonly id: string }>>;
  setSshCredential: (deviceId: string, credential: SshCredentialInput) => Promise<HubResult<unknown>>;
}

export interface AddedDevice {
  readonly id: string;
  /** Set when the device exists but its credential was not stored; the operator must enter it again. */
  readonly credentialError: HubError | null;
}

/**
 * The site's ID, creating the site first when it is new (VPN-002). A site with the same subnet is used
 * instead, so a save that failed after making the site does not fail again on an overlap.
 */
async function siteFor(placement: SitePlacement, api: AddDeviceApi): Promise<HubResult<string>> {
  if (placement.kind === 'existing') return { ok: true, value: placement.id };
  const subnet = placement.site.subnet.trim();
  const known = await api.sites();
  if (!known.ok) return known;
  const same = known.value.find((site) => site.subnet === subnet);
  if (same !== undefined) return { ok: true, value: same.id };
  const created = await api.createSite(placement.site.name.trim(), subnet);
  if (created.ok) return { ok: true, value: created.value.id };
  const { error } = created;
  return { ok: false, error: error.code === 'VALIDATION' ? { ...error, fields: siteFieldErrors(error.fields) } : error };
}

/**
 * D-6: places the device in its site (made first when new), creates the device with its default check,
 * then stores the typed credential. A failed credential does not undo the device: the result is still
 * `ok` with the ID and the error, so the route can open the device and say what is missing.
 */
export async function addDevice(
  values: DeviceFormValues,
  placement: SitePlacement,
  api: AddDeviceApi,
): Promise<HubResult<AddedDevice>> {
  const siteId = await siteFor(placement, api);
  if (!siteId.ok) return siteId;
  const created = await api.createDevice(createInput(values, siteId.value));
  if (!created.ok) return created;
  const { id } = created.value;
  const credential = credentialInput(values);
  if (credential === null) return { ok: true, value: { id, credentialError: null } };
  const stored = await api.setSshCredential(id, credential);
  return { ok: true, value: { id, credentialError: stored.ok ? null : stored.error } };
}
