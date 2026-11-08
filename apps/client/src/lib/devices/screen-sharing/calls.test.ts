import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Capabilities } from '$lib/platform/platform';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import { fakeHub } from '../hub.fixture';
import { screenSharingCalls } from './calls';

const shell = vi.hoisted(() => ({ answers: undefined as Capabilities | undefined, invoke: vi.fn() }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => shell.answers }));
// A shell is present, so only the platform's answers decide what the calls do.
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: shell.invoke }));

const MAC = { name: 'Studio', user: 'stephen', os: 'MACOS', vpnAddress: '100.64.0.7', sshListening: false };

beforeEach(() => {
  shell.invoke.mockReset().mockResolvedValue(MAC);
});

describe('screenSharingCalls (R-079)', () => {
  it('R-079: in the desktop app on a Mac the flow reads this Mac and may open Sharing', async () => {
    shell.answers = DESKTOP_MAC;
    const calls = screenSharingCalls(fakeHub().hub);
    expect(calls.sharingAvailable()).toBe(true);
    await expect(calls.thisMachine()).resolves.toEqual(MAC);
    expect(shell.invoke).toHaveBeenCalledWith('this_machine_read');
  });

  it('R-079, CLI-001: with the browser answers Sharing is not offered and this computer is not read', async () => {
    shell.answers = BROWSER;
    const calls = screenSharingCalls(fakeHub().hub);
    expect(calls.sharingAvailable()).toBe(false);
    await expect(calls.thisMachine()).resolves.toBeNull();
    expect(shell.invoke).not.toHaveBeenCalled();
  });

  it('R-059, R-079: on a Linux desktop this computer is read, but there is no Sharing to open', async () => {
    shell.answers = { ...DESKTOP_MAC, systemSettings: false };
    const calls = screenSharingCalls(fakeHub().hub);
    expect(calls.sharingAvailable()).toBe(false);
    await expect(calls.thisMachine()).resolves.toEqual(MAC);
  });
});
