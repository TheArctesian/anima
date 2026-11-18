import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readCloudDevice } from './device';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const DEVICE = { name: 'stephen-s-macbook-pro', computer: "Stephen's MacBook Pro", os: 'MACOS' };

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});

describe('readCloudDevice (HMD-044)', () => {
  it('gives the link name, the computer name and the OS that the shell reads', async () => {
    shell.invoke.mockResolvedValue({ ...DEVICE, extra: 'ignored' });
    expect(await readCloudDevice()).toEqual(DEVICE);
    expect(shell.invoke).toHaveBeenCalledExactlyOnceWith('cloud_device');
  });

  it('a computer with no name is still a device', async () => {
    shell.invoke.mockResolvedValue({ ...DEVICE, computer: '' });
    expect(await readCloudDevice()).toEqual({ ...DEVICE, computer: '' });
  });

  it.each([
    ['no answer', null],
    ['text', 'stephen-s-macbook-pro'],
    ['no link name', { ...DEVICE, name: '' }],
    ['a link name that is not text', { ...DEVICE, name: 7 }],
    ['no computer name field', { name: DEVICE.name, os: 'MACOS' }],
    ['an OS the app does not run on', { ...DEVICE, os: 'WINDOWS' }],
  ])('an answer with %s is not a device', async (_what, answer) => {
    shell.invoke.mockResolvedValue(answer);
    expect(await readCloudDevice()).toBeNull();
  });

  it('a shell that refuses gives nothing', async () => {
    shell.invoke.mockRejectedValue({ code: 'caller' });
    expect(await readCloudDevice()).toBeNull();
  });

  it('outside the desktop app the shell is not asked', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await readCloudDevice()).toBeNull();
    expect(shell.invoke).not.toHaveBeenCalled();
  });
});
