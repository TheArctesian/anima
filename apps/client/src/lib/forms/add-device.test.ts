import { expect, it, vi } from 'vitest';
import type { HubError, HubResult } from '$lib/hub/error';
import { addDevice, type AddDeviceApi } from './add-device';
import { formValues, type DeviceFormValues } from './device/values';

const FAILURE: HubError = { code: 'UNAVAILABLE', message: 'The hub is busy.' };

function values(change: Partial<DeviceFormValues['ssh']>, password = ''): DeviceFormValues {
  const base = formValues(null);
  return {
    ...base,
    name: 'nas',
    host: 'nas.home.arpa',
    ssh: { ...base.ssh, enabled: true, username: 'root', ...change },
    credential: { ...base.credential, password },
  };
}

function api(created: HubResult<{ id: string }>, stored: HubResult<unknown> = { ok: true, value: undefined }) {
  const calls = { createDevice: vi.fn(() => Promise.resolve(created)), setSshCredential: vi.fn(() => Promise.resolve(stored)) };
  return { calls, api: calls satisfies AddDeviceApi };
}

it('DEV-010, D-6: the device is created with the default check, then the credential is stored for its ID', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } });

  const result = await addDevice(values({}, 's3cret'), 'site-1', hub);

  expect(result).toEqual({ ok: true, value: { id: 'd1', credentialError: null } });
  expect(calls.createDevice).toHaveBeenCalledWith(expect.objectContaining({ siteId: 'site-1', addDefaultCheck: true }));
  expect(calls.setSshCredential).toHaveBeenCalledExactlyOnceWith('d1', { password: 's3cret' });
});

it('D-6: without a typed credential none is stored', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } });

  await addDevice(values({}), 'site-1', hub);

  expect(calls.setSshCredential).not.toHaveBeenCalled();
});

it('D-6: a failed create reports the error and stores no credential', async () => {
  const { calls, api: hub } = api({ ok: false, error: FAILURE });

  const result = await addDevice(values({}, 's3cret'), 'site-1', hub);

  expect(result).toEqual({ ok: false, error: FAILURE });
  expect(calls.setSshCredential).not.toHaveBeenCalled();
});

it('D-6: a credential that fails to save after the device exists still returns the ID with the error', async () => {
  const { api: hub } = api({ ok: true, value: { id: 'd1' } }, { ok: false, error: FAILURE });

  const result = await addDevice(values({}, 's3cret'), 'site-1', hub);

  expect(result).toEqual({ ok: true, value: { id: 'd1', credentialError: FAILURE } });
});
