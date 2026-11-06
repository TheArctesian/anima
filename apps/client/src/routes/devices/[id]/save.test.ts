import { describe, expect, it, vi } from 'vitest';
import { formValues } from '$lib/forms/device/values';
import { checkValues } from '$lib/forms/check/model/values';
import type { HubError } from '$lib/hub/hub';
import { check, device } from './page.fixture.svelte';
import { saveCheck, saveDevice, type CheckSaveCalls, type DeviceSaveCalls } from './save';

const LOCKED: HubError = { code: 'UNAVAILABLE', message: 'The key store is locked.' };

function deviceCalls(change: Partial<DeviceSaveCalls> = {}) {
  return {
    update: vi.fn<DeviceSaveCalls['update']>((_id, input) => Promise.resolve({ ok: true, value: device({ ...input } as never) })),
    setSshCredential: vi.fn<DeviceSaveCalls['setSshCredential']>(() => Promise.resolve({ ok: true, value: { credentialStored: true } })),
    ...change,
  };
}

describe('saveDevice (D-2, D-4)', () => {
  it('D-2: an unchanged form sends nothing', async () => {
    const calls = deviceCalls();
    const saved = await saveDevice(device(), formValues(device()), calls);
    expect(calls.update).not.toHaveBeenCalled();
    expect(calls.setSshCredential).not.toHaveBeenCalled();
    expect(saved.result.ok).toBe(true);
  });

  it('D-4: a typed password goes to setSshCredential after the update, and the device says it is stored', async () => {
    const calls = deviceCalls();
    const stored = device({}, { credentialStored: false });
    const values = { ...formValues(stored), name: 'vault', credential: { password: 'hunter2', privateKey: '', passphrase: '' } };
    const saved = await saveDevice(stored, values, calls);
    expect(calls.update).toHaveBeenCalledWith(stored.id, { name: 'vault' });
    expect(calls.setSshCredential).toHaveBeenCalledWith(stored.id, { password: 'hunter2' });
    expect(saved.device.ssh.credentialStored).toBe(true);
  });

  it('D-4: a failed credential keeps the updated device and gives the form the error', async () => {
    const calls = deviceCalls({ setSshCredential: () => Promise.resolve({ ok: false, error: LOCKED }) });
    const values = { ...formValues(device()), name: 'vault', credential: { password: 'hunter2', privateKey: '', passphrase: '' } };
    const saved = await saveDevice(device(), values, calls);
    expect(saved.device.name).toBe('vault');
    expect(saved.result).toEqual({ ok: false, error: LOCKED });
  });

  it('D-2: a failed update sends no credential', async () => {
    const calls = deviceCalls({ update: () => Promise.resolve({ ok: false, error: LOCKED }) });
    const values = { ...formValues(device()), name: 'vault', credential: { password: 'hunter2', privateKey: '', passphrase: '' } };
    const saved = await saveDevice(device(), values, calls);
    expect(calls.setSshCredential).not.toHaveBeenCalled();
    expect(saved.device.name).toBe('nas');
  });
});

describe('saveDevice remote passwords (DEV-011, H-2)', () => {
  const typed = (rdpPassword: string, vncPassword: string) => {
    const initial = formValues(device());
    return {
      ...initial,
      rdp: { ...initial.rdp, enabled: true, username: 'admin' },
      vnc: { ...initial.vnc, enabled: true },
      remoteSecrets: { ...initial.remoteSecrets, rdpPassword, vncPassword },
    };
  };
  const remoteCalls = (change: Partial<DeviceSaveCalls> = {}) => ({
    ...deviceCalls(),
    setRdpCredential: vi.fn<NonNullable<DeviceSaveCalls['setRdpCredential']>>(() => Promise.resolve({ ok: true, value: null })),
    setVncCredential: vi.fn<NonNullable<DeviceSaveCalls['setVncCredential']>>(() => Promise.resolve({ ok: true, value: null })),
    ...change,
  });

  it('DEV-011: typed RDP and VNC passwords are stored after the update', async () => {
    const calls = remoteCalls();
    const saved = await saveDevice(device(), typed('rdp-secret', 'vnc-secret'), calls);
    expect(calls.setRdpCredential).toHaveBeenCalledWith(device().id, { password: 'rdp-secret' });
    expect(calls.setVncCredential).toHaveBeenCalledWith(device().id, { password: 'vnc-secret' });
    expect(saved.result.ok).toBe(true);
  });

  it('DEV-011: a form with no typed remote password stores none', async () => {
    const calls = remoteCalls();
    await saveDevice(device(), typed('', ''), calls);
    expect(calls.setRdpCredential).not.toHaveBeenCalled();
    expect(calls.setVncCredential).not.toHaveBeenCalled();
  });

  it('H-2: a typed password with no call to store it is a failure, not a success', async () => {
    const calls = deviceCalls();
    const saved = await saveDevice(device(), typed('rdp-secret', ''), calls);
    expect(saved.result.ok).toBe(false);
    expect(JSON.stringify(saved.result)).toContain('cannot store this password yet');
    expect(JSON.stringify(saved.result)).not.toContain('rdp-secret');
  });

  it('DEV-011: a failed RDP password is reported, and the VNC password is still stored', async () => {
    const calls = remoteCalls({ setRdpCredential: () => Promise.resolve({ ok: false, error: LOCKED }) });
    const saved = await saveDevice(device(), typed('rdp-secret', 'vnc-secret'), calls);
    expect(saved.result).toEqual({ ok: false, error: LOCKED });
    expect(calls.setVncCredential).toHaveBeenCalledOnce();
  });

  it('DEV-011: a failed VNC password is reported after a stored RDP password', async () => {
    const calls = remoteCalls({ setVncCredential: () => Promise.resolve({ ok: false, error: LOCKED }) });
    const saved = await saveDevice(device(), typed('rdp-secret', 'vnc-secret'), calls);
    expect(calls.setRdpCredential).toHaveBeenCalledOnce();
    expect(saved.result).toEqual({ ok: false, error: LOCKED });
  });

  it('D-2: an untouched form sends no remote password call', async () => {
    const calls = remoteCalls();
    await saveDevice(device(), formValues(device()), calls);
    expect(calls.update).not.toHaveBeenCalled();
    expect(calls.setRdpCredential).not.toHaveBeenCalled();
    expect(calls.setVncCredential).not.toHaveBeenCalled();
  });
});

describe('saveCheck (Section 6.4)', () => {
  const calls = (): CheckSaveCalls & { create: ReturnType<typeof vi.fn>; update: ReturnType<typeof vi.fn> } => ({
    create: vi.fn(() => Promise.resolve({ ok: true as const, value: check({ id: 'c2' }) })),
    update: vi.fn(() => Promise.resolve({ ok: true as const, value: check() })),
  });

  it('6.4: a new check is created on the device with every common setting', async () => {
    const api = calls();
    await saveCheck('d1', null, { ...checkValues(null, 'TCP'), name: 'ssh port' }, api);
    expect(api.create).toHaveBeenCalledWith(expect.objectContaining({ deviceId: 'd1', name: 'ssh port', intervalSeconds: 60, retries: 3 }));
  });

  it('6.4: an unchanged check sends nothing; a change sends only that field', async () => {
    const api = calls();
    const saved = check();
    expect(await saveCheck('d1', saved, checkValues(saved, saved.type), api)).toEqual({ ok: true, value: saved });
    expect(api.update).not.toHaveBeenCalled();
    await saveCheck('d1', saved, { ...checkValues(saved, saved.type), retries: '5' }, api);
    expect(api.update).toHaveBeenCalledWith('c1', { retries: 5 });
  });
});
