import type { DeviceCheck } from '$lib/checks/api';
import type { DeviceDetail, StoredCredential } from '$lib/devices/api';
import { createCheckInput, updateCheckInput } from '$lib/forms/check/model/input';
import { checkValues, type CheckFormValues } from '$lib/forms/check/model/values';
import { credentialInput, updateInput } from '$lib/forms/device/input';
import { formValues, type DeviceFormValues } from '$lib/forms/device/values';
import type { HubResult } from '$lib/hub/hub';
import type { CreateCheckInput, SshCredentialInput, UpdateCheckInput, UpdateDeviceInput } from '$lib/hub/generated/graphql';

/** The device calls a save needs; the route passes `devicesApi(hub)`. */
export interface DeviceSaveCalls {
  update: (id: string, input: UpdateDeviceInput) => Promise<HubResult<DeviceDetail>>;
  setSshCredential: (id: string, credential: SshCredentialInput) => Promise<HubResult<StoredCredential>>;
}

/** The device after a save, even a half-failed one, and what the form shows. */
export interface DeviceSave {
  readonly device: DeviceDetail;
  readonly result: HubResult<unknown>;
}

/**
 * D-2, D-4: sends only the changed fields, then a newly typed credential. A failed credential keeps the updated
 * device, so the page shows what the hub holds, and the form shows the error. An unchanged form sends nothing.
 */
export async function saveDevice(saved: DeviceDetail, values: DeviceFormValues, calls: DeviceSaveCalls): Promise<DeviceSave> {
  const input = updateInput(formValues(saved), values);
  let device = saved;
  if (Object.keys(input).length > 0) {
    const updated = await calls.update(saved.id, input);
    if (!updated.ok) return { device, result: updated };
    device = updated.value;
  }
  const credential = credentialInput(values);
  if (credential === null) return { device, result: { ok: true, value: device } };
  const stored = await calls.setSshCredential(saved.id, credential);
  if (!stored.ok) return { device, result: stored };
  device = { ...device, ssh: { ...device.ssh, credentialStored: stored.value.credentialStored } };
  return { device, result: { ok: true, value: device } };
}

/** The check calls a save needs; the route passes `checksApi(hub)`. */
export interface CheckSaveCalls {
  create: (input: CreateCheckInput) => Promise<HubResult<DeviceCheck>>;
  update: (id: string, input: UpdateCheckInput) => Promise<HubResult<DeviceCheck>>;
}

/** Section 6.4: creates the check, or sends only what changed; an unchanged form answers the check as it is. */
export function saveCheck(
  deviceId: string,
  check: DeviceCheck | null,
  values: CheckFormValues,
  calls: CheckSaveCalls,
): Promise<HubResult<DeviceCheck>> {
  if (check === null) return calls.create(createCheckInput(values, deviceId));
  const input = updateCheckInput(checkValues(check, check.type), values);
  if (Object.keys(input).length === 0) return Promise.resolve({ ok: true, value: check });
  return calls.update(check.id, input);
}
