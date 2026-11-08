import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readThisMachine, thisMachineFrom, type ThisMachine } from '$lib/desktop/this-machine';
import { validateDevice } from '../form/validate';
import { defaultRemoteValues } from '../form/remote/values';
import { thisMachineValues } from '../form/values';
import { DONT_ASK_KEY, ThisMachinePrompt, dontAskLabel, machineToOffer, promptTitle, thisMachineLabel } from './this-machine';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const MAC: ThisMachine = { name: 'Studio', user: 'stephen', os: 'MACOS', vpnAddress: '100.64.0.7', sshListening: true };

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});

/** A `Storage` held in a map; `refuse` makes every call throw, as a privacy mode does. */
function memoryStorage(refuse = false): Storage {
  const entries = new Map<string, string>();
  const guard = () => {
    if (refuse) throw new DOMException('denied', 'SecurityError');
  };
  return {
    get length() {
      return entries.size;
    },
    clear: () => {
      entries.clear();
    },
    key: (index) => [...entries.keys()][index] ?? null,
    getItem: (key) => {
      guard();
      return entries.get(key) ?? null;
    },
    setItem: (key, value) => {
      guard();
      entries.set(key, value);
    },
    removeItem: (key) => {
      entries.delete(key);
    },
  };
}

describe('readThisMachine', () => {
  it('R-060: asks the shell with no arguments and returns what it read', async () => {
    shell.invoke.mockResolvedValue({ ...MAC });
    expect(await readThisMachine()).toEqual(MAC);
    expect(shell.invoke).toHaveBeenCalledWith('this_machine_read');
  });

  it('R-060: is null outside the desktop app, without asking the shell', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await readThisMachine()).toBeNull();
    expect(shell.invoke).not.toHaveBeenCalled();
  });

  it('R-060: is null when the shell fails or the computer is not on the VPN', async () => {
    shell.invoke.mockRejectedValue({ code: 'failed', message: 'this computer could not be read' });
    expect(await readThisMachine()).toBeNull();
    shell.invoke.mockResolvedValue({ ...MAC, vpnAddress: null, sshListening: false });
    expect(await readThisMachine()).toBeNull();
  });
});

describe('thisMachineFrom', () => {
  it('R-060: keeps only the five known fields', () => {
    expect(thisMachineFrom({ ...MAC, extra: 'dropped' })).toEqual(MAC);
  });

  it('R-060: refuses an answer of the wrong shape', () => {
    for (const answer of [
      null,
      'Studio',
      { ...MAC, os: 'WINDOWS' },
      { ...MAC, name: 7 },
      { ...MAC, user: undefined },
      { ...MAC, sshListening: 'yes' },
      { ...MAC, vpnAddress: '100.64.0' },
      { ...MAC, vpnAddress: '100.64.0.256' },
      { ...MAC, vpnAddress: '100.128.0.1' },
      { ...MAC, vpnAddress: '192.168.1.20' },
      { ...MAC, vpnAddress: '100.64.0.7; rm -rf /' },
    ]) {
      expect(thisMachineFrom(answer)).toBeNull();
    }
  });
});

describe('thisMachineValues', () => {
  it('R-060: fills every field the shell knows, as a workstation with SSH by password and no password', () => {
    expect(thisMachineValues(MAC)).toEqual({
      name: 'Studio',
      host: '100.64.0.7',
      kind: 'workstation',
      os: 'MACOS',
      tags: [],
      ssh: { enabled: true, port: '22', username: 'stephen', auth: 'PASSWORD' },
      credential: { password: '', privateKey: '', passphrase: '' },
      remoteSecrets: { rdpPassword: '', vncPassword: '' },
      ...defaultRemoteValues(),
    });
  });

  it('R-060: SSH is on only when it answered, and the values pass validation as they are', () => {
    const off = thisMachineValues({ ...MAC, os: 'LINUX', sshListening: false });
    expect(off.ssh.enabled).toBe(false);
    expect(off.os).toBe('LINUX');
    expect(validateDevice(thisMachineValues(MAC))).toEqual([]);
    expect(validateDevice(off)).toEqual([]);
  });
});

describe('machineToOffer', () => {
  const read = (machine: ThisMachine | null) => () => Promise.resolve(machine);
  const hosts = (list: readonly string[]) => vi.fn(() => Promise.resolve({ ok: true as const, value: list }));

  it('R-060: offers this computer when no device has its VPN address', async () => {
    expect(await machineToOffer(read(MAC), hosts(['nas.lan', '100.64.0.8']))).toEqual({ kind: 'offer', machine: MAC });
  });

  it('R-060: a device whose host is the VPN address is this computer, so nothing is offered', async () => {
    expect(await machineToOffer(read(MAC), hosts(['nas.lan', '100.64.0.7']))).toEqual({ kind: 'device' });
    expect(await machineToOffer(read(MAC), hosts([' 100.64.0.7 ']))).toEqual({ kind: 'device' });
  });

  it('R-060: nothing is offered for an unknown computer, and the hub is not asked', async () => {
    const known = hosts([]);
    expect(await machineToOffer(read(null), known)).toEqual({ kind: 'unknown' });
    expect(known).not.toHaveBeenCalled();
  });

  it('R-060: nothing is offered when the hosts cannot be read, since it might be a duplicate', async () => {
    const failed = () => Promise.resolve({ ok: false as const, error: { code: 'unreachable' as const, message: 'No answer.' } });
    expect(await machineToOffer(read(MAC), failed)).toEqual({ kind: 'unknown' });
  });
});

describe('labels', () => {
  it('R-060: a Mac is "This Mac"; Linux is "This computer"', () => {
    expect([thisMachineLabel('MACOS'), promptTitle('MACOS'), dontAskLabel('MACOS')]).toEqual([
      'This Mac',
      'Add this Mac?',
      "Don't ask again on this Mac",
    ]);
    expect([thisMachineLabel('LINUX'), promptTitle('LINUX'), dontAskLabel('LINUX')]).toEqual([
      'This computer',
      'Add this computer?',
      "Don't ask again on this computer",
    ]);
  });
});

describe('ThisMachinePrompt', () => {
  it('R-060: asks once per app start; "Not now" asks again at the next start', () => {
    const storage = memoryStorage();
    const run = new ThisMachinePrompt(() => storage);
    expect(run.due).toBe(true);
    run.asked();
    expect(run.due).toBe(false);
    expect(new ThisMachinePrompt(() => storage).due).toBe(true);
  });

  it('R-060: "Don\'t ask again" is kept under a namespaced key and holds across starts', () => {
    const storage = memoryStorage();
    new ThisMachinePrompt(() => storage).turnOff();
    expect(storage.getItem(DONT_ASK_KEY)).toBe('true');
    expect(DONT_ASK_KEY.startsWith('anima.')).toBe(true);
    expect(new ThisMachinePrompt(() => storage).due).toBe(false);
  });

  it('R-060: once this computer is found to be a device, the rest of the run does not look again', () => {
    const storage = memoryStorage();
    const run = new ThisMachinePrompt(() => storage);
    run.isADevice();
    expect(run.due).toBe(false);
    expect(new ThisMachinePrompt(() => storage).due).toBe(true);
  });

  it('R-060: a storage that refuses or is missing never throws; the prompt stops for this run', () => {
    for (const storage of [memoryStorage(true), null]) {
      const run = new ThisMachinePrompt(() => storage);
      expect(run.due).toBe(true);
      expect(() => {
        run.turnOff();
      }).not.toThrow();
      expect(run.due).toBe(false);
    }
  });
});
