// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../../../forms/animations.fixture';
import { addDevice, type AddDeviceApi } from '../../add/add-device';
import type { RemoteDeviceFields } from './values';
import { OK, SAVED, input, leave, save, show, type } from '../form/form.fixture';
import type { DeviceFields, DeviceFormValues } from '../values';
import { formValues } from '../values';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

beforeEach(() => {
  installAnimations();
});

afterEach(leave);

const K1: RemoteDeviceFields = {
  rdp: {
    enabled: true,
    port: 3389,
    username: 'admin',
    domain: null,
    security: 'NLA',
    ignoreCertificate: false,
    resizeMethod: 'DISPLAY_UPDATE',
    audioOut: true,
    microphone: false,
    sharedDrive: false,
    credentialStored: true,
    serverCertificate: { fingerprintSha256: 'AA:BB', subject: 'CN=desk', notAfter: '2027-01-02T00:00:00Z', firstSeenAt: '2026-11-04T09:00:00Z' },
  },
  vnc: { enabled: true, port: 5900, username: null, credentialStored: true },
  macAddress: null,
  metricsSource: 'NONE',
  transfer: { upload: true, download: true, copy: true, paste: true },
  accessWarnings: ['UNENCRYPTED_VNC'],
};
const WITH_K1: DeviceFields = { ...SAVED, ...K1 };

/** The disclosure button of `label`; its text starts with an icon. */
function disclosure(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.includes(label));
  if (!found) throw new Error(`no disclosure ${label}`);
  return found;
}

function checkbox(label: string): HTMLInputElement {
  const found = [...document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].find((box) =>
    box.parentElement?.textContent.includes(label),
  );
  if (!found) throw new Error(`no checkbox ${label}`);
  return found;
}

it('RA-030: RDP and VNC fields appear when enabled and are hidden when off', () => {
  show(SAVED);
  disclosure('Remote Desktop (RDP)').click();
  disclosure('VNC').click();
  flushSync();
  expect(document.getElementById('device-rdp-port')).toBeNull();
  expect(document.getElementById('device-vnc-port')).toBeNull();

  checkbox('RDP enabled').click();
  checkbox('VNC enabled').click();
  flushSync();

  expect(input('rdp-port').value).toBe('3389');
  expect(input('vnc-port').value).toBe('5900');
});

it('DEV-011: the RDP and VNC passwords are hidden text that the browser does not remember', () => {
  show(WITH_K1, { section: 'rdp' });
  disclosure('VNC').click();
  flushSync();
  for (const id of ['rdp-password', 'vnc-password']) {
    expect(input(id).type).toBe('password');
    expect(input(id).autocomplete).toBe('new-password');
    expect(input(id).value).toBe('');
  }
});

it('RA-020: ignoring the certificate or choosing standard RDP security shows a warning', () => {
  show(WITH_K1, { section: 'rdp' });
  expect(document.querySelector('[data-section="rdp"] .warning')).toBeNull();

  const ignore = checkbox('Ignore the server certificate');
  ignore.click();
  flushSync();
  expect(document.body.textContent).toContain('Ignoring the certificate');

  ignore.click();
  const security = document.getElementById('device-rdp-security') as HTMLSelectElement;
  security.value = 'RDP';
  security.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
  expect(document.body.textContent).toContain('Standard RDP security is weak');
});

it('RA-020: the hub UNENCRYPTED_VNC warning is shown, and the recorded RDP certificate is read-only text', () => {
  show(WITH_K1, { section: 'rdp' });
  disclosure('VNC').click();
  flushSync();
  expect(document.body.textContent).toContain('This VNC connection is not encrypted');
  const certificate = document.querySelector('[aria-label="Server certificate"]');
  expect(certificate?.textContent).toContain('AA:BB');
  expect(certificate?.textContent).toContain('CN=desk');
  expect(certificate?.textContent).toContain('2027-01-02');
  expect(certificate?.querySelector('input')).toBeNull();
});

it('D-4: the Remove credential buttons appear only when the route gives the callbacks', () => {
  show(WITH_K1, { section: 'rdp', rdpCredentialStored: true, vncCredentialStored: true });
  disclosure('VNC').click();
  flushSync();
  expect(document.body.textContent).toContain('Credential stored');
  expect([...document.querySelectorAll('button')].filter((b) => b.textContent.trim() === 'Remove credential')).toHaveLength(0);
  leave();

  show(WITH_K1, {
    section: 'rdp',
    rdpCredentialStored: true,
    vncCredentialStored: true,
    onClearRdpCredential: () => Promise.resolve(OK),
    onClearVncCredential: () => Promise.resolve(OK),
  });
  disclosure('VNC').click();
  flushSync();
  expect([...document.querySelectorAll('button')].filter((b) => b.textContent.trim() === 'Remove credential')).toHaveLength(2);
});

it('RA-031: a bad MAC address is not saved and the open disclosure focuses it', async () => {
  const { onSave } = show(SAVED);
  type('macAddress', 'not-a-mac');
  await save();
  expect(onSave).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(input('macAddress'));
});

it('RA-030: an RDP error opens the closed RDP disclosure and focuses the field', async () => {
  const { onSave } = show({ ...SAVED, ...K1, rdp: { ...K1.rdp, port: 0 } });
  expect(input('rdp-port').closest('[hidden]')).not.toBeNull();

  await save();

  expect(onSave).not.toHaveBeenCalled();
  expect(input('rdp-port').closest('[hidden]')).toBeNull();
  expect(document.activeElement).toBe(input('rdp-port'));
});

it('DEV-011: typed RDP and VNC passwords go to save and are cleared after it', async () => {
  const { onSave } = show(WITH_K1, { section: 'rdp' });
  disclosure('VNC').click();
  flushSync();
  type('rdp-password', 'secret-r');
  type('vnc-password', 'secret-v');
  await save();

  const [values] = onSave.mock.calls[0] ?? [];
  expect(values?.remoteSecrets).toEqual({ rdpPassword: 'secret-r', vncPassword: 'secret-v' });
  expect(input('rdp-password').value).toBe('');
  expect(input('vnc-password').value).toBe('');
});

function fakeApi(change: Partial<AddDeviceApi> = {}): AddDeviceApi {
  return {
    sites: () => Promise.resolve({ ok: true, value: [] }),
    createSite: () => Promise.resolve({ ok: true, value: { id: 's1' } }),
    createDevice: () => Promise.resolve({ ok: true, value: { id: 'd1' } }),
    setSshCredential: () => Promise.resolve(OK),
    setRdpCredential: () => Promise.resolve({ ok: true, value: { credentialStored: true } }),
    setVncCredential: () => Promise.resolve({ ok: true, value: { credentialStored: true } }),
    createCheck: () => Promise.resolve(OK),
    ...change,
  };
}

function withRdpPassword(): DeviceFormValues {
  const base = formValues(null);
  return {
    ...base,
    name: 'pc',
    host: '10.0.0.9',
    rdp: { ...base.rdp, enabled: true, username: 'me' },
    remoteSecrets: { rdpPassword: 'typed', vncPassword: '' },
  };
}

it('DEV-011: addDevice stores a typed RDP password through setRdpCredential', async () => {
  const setRdpCredential = vi.fn(() => Promise.resolve({ ok: true as const, value: { credentialStored: true } }));
  const result = await addDevice(withRdpPassword(), { kind: 'existing', id: 's1' }, fakeApi({ setRdpCredential }));
  expect(setRdpCredential).toHaveBeenCalledWith('d1', { password: 'typed' });
  expect(result).toEqual({ ok: true, value: { id: 'd1', credentialError: null } });
});

it('DEV-011: addDevice reports an RDP password it could not store', async () => {
  const failed = await addDevice(
    withRdpPassword(),
    { kind: 'existing', id: 's1' },
    fakeApi({ setRdpCredential: () => Promise.resolve({ ok: false, error: { code: 'INTERNAL', message: 'vault locked' } }) }),
  );
  expect(failed.ok && failed.value.credentialError?.message).toBe('vault locked');
});

it('RA-032, RA-033: the Files and clipboard flags default on and a cleared one reaches the saved values', async () => {
  const { onSave } = show(SAVED);
  disclosure('Files and clipboard').click();
  flushSync();
  for (const label of ['upload', 'download', 'copy', 'paste']) expect(checkbox(`Allow ${label}`).checked).toBe(true);

  checkbox('Allow paste').click();
  flushSync();
  await save();

  const [values] = onSave.mock.calls[0] ?? [];
  expect(values?.transfer).toEqual({ upload: true, download: true, copy: true, paste: false });
});
