import { expect, it, vi } from 'vitest';
import type { HubError, HubResult } from '$lib/hub/error';
import { addDevice, type AddDeviceApi } from './add-device';
import { VPN_SITE, type SitePlacement } from './device/site';
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

const HOME: SitePlacement = { kind: 'existing', id: 'site-1' };
const NEW_VPN: SitePlacement = { kind: 'new', site: { ...VPN_SITE } };

function api(
  created: HubResult<{ id: string }>,
  stored: HubResult<unknown> = { ok: true, value: undefined },
  sites: { id: string; subnet: string }[] = [],
  site: HubResult<{ id: string }> = { ok: true, value: { id: 'site-new' } },
) {
  const calls = {
    sites: vi.fn(() => Promise.resolve({ ok: true as const, value: sites })),
    createSite: vi.fn(() => Promise.resolve(site)),
    createDevice: vi.fn(() => Promise.resolve(created)),
    setSshCredential: vi.fn(() => Promise.resolve(stored)),
  };
  return { calls, api: calls satisfies AddDeviceApi };
}

it('DEV-010, D-6: the device is created with the default check, then the credential is stored for its ID', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } });

  const result = await addDevice(values({}, 's3cret'), HOME, hub);

  expect(result).toEqual({ ok: true, value: { id: 'd1', credentialError: null } });
  expect(calls.createDevice).toHaveBeenCalledWith(expect.objectContaining({ siteId: 'site-1', addDefaultCheck: true }));
  expect(calls.setSshCredential).toHaveBeenCalledExactlyOnceWith('d1', { password: 's3cret' });
});

it('D-6: without a typed credential none is stored', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } });

  await addDevice(values({}), HOME, hub);

  expect(calls.setSshCredential).not.toHaveBeenCalled();
});

it('D-6: a failed create reports the error and stores no credential', async () => {
  const { calls, api: hub } = api({ ok: false, error: FAILURE });

  const result = await addDevice(values({}, 's3cret'), HOME, hub);

  expect(result).toEqual({ ok: false, error: FAILURE });
  expect(calls.setSshCredential).not.toHaveBeenCalled();
});

it('D-6: a credential that fails to save after the device exists still returns the ID with the error', async () => {
  const { api: hub } = api({ ok: true, value: { id: 'd1' } }, { ok: false, error: FAILURE });

  const result = await addDevice(values({}, 's3cret'), HOME, hub);

  expect(result).toEqual({ ok: true, value: { id: 'd1', credentialError: FAILURE } });
});

it('VPN-002: when the hub has no site yet, the new site is created first and the device goes in it', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } });

  await addDevice(values({}), NEW_VPN, hub);

  expect(calls.createSite).toHaveBeenCalledExactlyOnceWith('VPN', '100.64.0.0/10');
  expect(calls.createDevice).toHaveBeenCalledWith(expect.objectContaining({ siteId: 'site-new' }));
});

it('VPN-002: a site with the same subnet, left by a save that failed later, is used instead of made again', async () => {
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } }, undefined, [{ id: 'site-vpn', subnet: '100.64.0.0/10' }]);

  await addDevice(values({}), NEW_VPN, hub);

  expect(calls.createSite).not.toHaveBeenCalled();
  expect(calls.createDevice).toHaveBeenCalledWith(expect.objectContaining({ siteId: 'site-vpn' }));
});

it('VPN-001: a site the hub refuses reports its fields under site. and creates no device', async () => {
  const refused: HubError = { code: 'VALIDATION', message: 'Invalid input.', fields: [{ field: 'subnet', message: 'This subnet overlaps an existing site subnet.' }] };
  const { calls, api: hub } = api({ ok: true, value: { id: 'd1' } }, undefined, [], { ok: false, error: refused });

  const result = await addDevice(values({}), NEW_VPN, hub);

  expect(result).toEqual({ ok: false, error: { ...refused, fields: [{ field: 'site.subnet', message: 'This subnet overlaps an existing site subnet.' }] } });
  expect(calls.createDevice).not.toHaveBeenCalled();
});
