import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readThisMachine, type ThisMachine } from '$lib/desktop/this-machine';
import { fakeHub } from '$lib/devices/hub.fixture';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const MAC: ThisMachine = { name: 'Studio', user: 'stephen', os: 'MACOS', vpnAddress: '100.64.0.7', sshListening: true };
const LOGIN_SERVER = 'https://hs.anima.ing';

/** A hub that names its VPN login server (ADR-005). */
function hub() {
  return fakeHub({ VpnLoginServer: { vpnLoginServer: LOGIN_SERVER } }).hub;
}

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});


describe('readThisMachine', () => {
  it.each(['MACOS', 'LINUX', 'WINDOWS'] as const)("R-060, ADR-005, WIN1: reads %s with the hub's login server and keeps only machine fields", async (os) => {
    const machine = { ...MAC, os };
    shell.invoke.mockResolvedValue({ ...machine, extra: 'dropped' });
    expect(await readThisMachine(hub())).toEqual(machine);
    expect(shell.invoke).toHaveBeenCalledWith('this_machine_read', { loginServer: LOGIN_SERVER });
  });

  it('ADR-005: is null without asking the shell when the hub does not name its login server', async () => {
    shell.invoke.mockResolvedValue(MAC);
    expect(await readThisMachine(fakeHub().hub)).toBeNull();
    expect(shell.invoke).not.toHaveBeenCalled();
  });

  it('R-060: is null outside the desktop app, without asking the shell', async () => {
    shell.isTauri.mockReturnValue(false);
    expect(await readThisMachine(hub())).toBeNull();
    expect(shell.invoke).not.toHaveBeenCalled();
  });

  it('R-060: is null when the shell fails or the computer is not on the VPN', async () => {
    shell.invoke.mockRejectedValue({ code: 'failed', message: 'this computer could not be read' });
    expect(await readThisMachine(hub())).toBeNull();
    shell.invoke.mockResolvedValue({ ...MAC, vpnAddress: null, sshListening: false });
    expect(await readThisMachine(hub())).toBeNull();
  });

  it.each([
    ['null', null],
    ['a string', 'Studio'],
    ['an unsupported OS', { ...MAC, os: 'ANDROID' }],
    ['a non-string name', { ...MAC, name: 7 }],
    ['a missing user', { ...MAC, user: undefined }],
    ['a non-boolean SSH flag', { ...MAC, sshListening: 'yes' }],
    ['an incomplete IPv4 address', { ...MAC, vpnAddress: '100.64.0' }],
    ['an out-of-range IPv4 octet', { ...MAC, vpnAddress: '100.64.0.256' }],
    ['an address just outside the VPN range', { ...MAC, vpnAddress: '100.128.0.1' }],
    ['a private LAN address', { ...MAC, vpnAddress: '192.168.1.20' }],
    ['an IPv4 address with trailing text', { ...MAC, vpnAddress: '100.64.0.7; rm -rf /' }],
  ])('R-060: refuses a shell answer with %s', async (_shape, answer) => {
    shell.invoke.mockResolvedValue(answer);
    expect(await readThisMachine(hub())).toBeNull();
  });
});
