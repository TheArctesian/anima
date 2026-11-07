import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import type { DeviceDetail } from './api';
import { devicesApi } from './client';
import { fakeHub } from './hub.fixture';

const DETAIL: DeviceDetail = {
  id: 'd1',
  name: 'nas',
  host: 'nas.lan',
  ip: '10.0.0.5',
  kind: 'server',
  os: 'LINUX',
  tags: ['lab'],
  maintenance: false,
  status: 'UP',
  site: { id: 's1', name: 'Home' },
  ssh: {
    enabled: true,
    port: 22,
    username: 'root',
    auth: 'PASSWORD',
    credentialStored: true,
    hostKey: { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T10:00:00Z' },
  },
  macAddress: null,
  webInterfaces: [],
  webInterfaceSuggestions: [],
  metricsSource: 'NONE',
  accessWarnings: [],
  rdp: {
    enabled: false,
    port: 3389,
    username: '',
    domain: null,
    security: 'ANY',
    ignoreCertificate: false,
    resizeMethod: 'DISPLAY_UPDATE',
    audioOut: true,
    microphone: false,
    sharedDrive: false,
    credentialStored: false,
    serverCertificate: null,
  },
  vnc: { enabled: false, port: 5900, username: null, credentialStored: false },
  transfer: { upload: true, download: true, copy: true, paste: true },
  day: 0.999,
  week: 0.99,
  month: null,
};

const DENIED: HubError = { code: 'VALIDATION', message: 'Check the form.', fields: [{ field: 'host', message: 'Not a host.' }] };

describe('devicesApi: device detail and form (API-008, CQ-045, DEV-010)', () => {
  it('DEV-020: get sends the ID and returns the device with its uptimes', async () => {
    const fake = fakeHub({ Device: { device: DETAIL } });
    expect(await devicesApi(fake.hub).get('d1')).toEqual({ ok: true, value: DETAIL });
    expect(fake.requests).toEqual([{ operation: 'Device', variables: { id: 'd1' } }]);
  });

  it('DEV-020: get returns null for a device the hub does not have', async () => {
    const fake = fakeHub({ Device: { device: null } });
    expect(await devicesApi(fake.hub).get('gone')).toEqual({ ok: true, value: null });
  });

  it('DEV-010: sites returns the site list for the picker', async () => {
    const sites = [{ id: 's1', name: 'Home' }];
    const fake = fakeHub({ Sites: { sites } });
    expect(await devicesApi(fake.hub).sites()).toEqual({ ok: true, value: sites });
  });

  it('DEV-010: create sends the input unchanged and returns the new device', async () => {
    const fake = fakeHub({ CreateDevice: { createDevice: DETAIL } });
    const input = { siteId: 's1', name: 'nas', host: 'nas.lan', addDefaultCheck: true };
    expect(await devicesApi(fake.hub).create(input)).toEqual({ ok: true, value: DETAIL });
    expect(fake.requests).toEqual([{ operation: 'CreateDevice', variables: { input } }]);
  });

  it('DEV-010, K7: a VALIDATION error from create keeps its field list', async () => {
    const fake = fakeHub({ CreateDevice: DENIED });
    expect(await devicesApi(fake.hub).create({ siteId: 's1', name: 'x', host: '!' })).toEqual({ ok: false, error: DENIED });
  });

  it('DEV-010: update sends the ID and only the changed fields', async () => {
    const fake = fakeHub({ UpdateDevice: { updateDevice: DETAIL } });
    expect(await devicesApi(fake.hub).update('d1', { name: 'nas2' })).toEqual({ ok: true, value: DETAIL });
    expect(fake.requests).toEqual([{ operation: 'UpdateDevice', variables: { id: 'd1', input: { name: 'nas2' } } }]);
  });

  it('DEV-014: delete sends the ID and returns the deleted ID', async () => {
    const fake = fakeHub({ DeleteDevice: { deleteDevice: 'd1' } });
    expect(await devicesApi(fake.hub).delete('d1')).toEqual({ ok: true, value: 'd1' });
    expect(fake.requests).toEqual([{ operation: 'DeleteDevice', variables: { id: 'd1' } }]);
  });

  it('DEV-011: setSshCredential sends the credential and returns whether one is stored', async () => {
    const fake = fakeHub({ SetSshCredential: { setSshCredential: { credentialStored: true } } });
    const credential = { privateKey: { key: '-----BEGIN OPENSSH PRIVATE KEY-----', passphrase: 'pw' } };
    expect(await devicesApi(fake.hub).setSshCredential('d1', credential)).toEqual({ ok: true, value: { credentialStored: true } });
    expect(fake.requests).toEqual([{ operation: 'SetSshCredential', variables: { deviceId: 'd1', credential } }]);
  });
});

describe('devicesApi: stored credentials (API-008, DEV-011)', () => {
  it('DEV-011: clearSshCredential sends the device ID and returns credentialStored false', async () => {
    const fake = fakeHub({ ClearSshCredential: { clearSshCredential: { credentialStored: false } } });
    expect(await devicesApi(fake.hub).clearSshCredential('d1')).toEqual({ ok: true, value: { credentialStored: false } });
    expect(fake.requests).toEqual([{ operation: 'ClearSshCredential', variables: { deviceId: 'd1' } }]);
  });

  it('RA-011: clearRdpCredential and clearVncCredential send the device ID and return credentialStored false', async () => {
    const fake = fakeHub({
      ClearRdpCredential: { clearRdpCredential: { credentialStored: false } },
      ClearVncCredential: { clearVncCredential: { credentialStored: false } },
    });
    const api = devicesApi(fake.hub);
    expect(await api.clearRdpCredential('d1')).toEqual({ ok: true, value: { credentialStored: false } });
    expect(await api.clearVncCredential('d1')).toEqual({ ok: true, value: { credentialStored: false } });
    expect(fake.requests).toEqual([
      { operation: 'ClearRdpCredential', variables: { deviceId: 'd1' } },
      { operation: 'ClearVncCredential', variables: { deviceId: 'd1' } },
    ]);
  });

  it('K7: NOT_FOUND from delete and an unreachable hub pass through', async () => {
    const missing: HubError = { code: 'NOT_FOUND', message: 'No such device.' };
    expect(await devicesApi(fakeHub({ DeleteDevice: missing }).hub).delete('x')).toEqual({ ok: false, error: missing });
    const silent = await devicesApi(fakeHub().hub).get('d1');
    expect(silent.ok).toBe(false);
  });
});
