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
