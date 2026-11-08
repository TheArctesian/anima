// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import RemoteLoginButton from '../devices/form/form/RemoteLoginButton.svelte';
import { openSharingSettings, SystemSettingsError } from './system-settings';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

describe('openSharingSettings', () => {
  it('R-059: invokes the shell command with no arguments, so the web view names no URL', async () => {
    shell.invoke.mockResolvedValue(null);
    await openSharingSettings();
    expect(shell.invoke).toHaveBeenCalledWith('sharing_settings_open');
  });

  it('R-059: shell codes become typed errors without the shell message', async () => {
    shell.invoke.mockRejectedValue({ code: 'unavailable', message: 'raw os text' });
    const failure = await openSharingSettings().catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(SystemSettingsError);
    expect(failure).toMatchObject({ problem: 'unavailable' });
    expect(String(failure)).not.toContain('raw os text');
    shell.invoke.mockRejectedValue('command not found');
    await expect(openSharingSettings()).rejects.toMatchObject({ problem: 'failed' });
  });
});

describe('RemoteLoginButton', () => {
  function show(available: boolean): HTMLButtonElement | null {
    app = mount(RemoteLoginButton, { target: document.body, props: { available } });
    flushSync();
    return document.querySelector('button');
  }

  it('R-059: is not shown where the settings cannot be opened', () => {
    expect(show(false)).toBeNull();
  });

  it('R-059: the button opens the Remote Login settings', async () => {
    shell.invoke.mockResolvedValue(null);
    const button = show(true);
    expect(button?.textContent).toBe('Open Remote Login settings');
    button?.click();
    await tick();
    expect(shell.invoke).toHaveBeenCalledWith('sharing_settings_open');
    expect(document.querySelector('[role="alert"]')).toBeNull();
  });

  it('R-059: a failure to open shows a fixed message', async () => {
    shell.invoke.mockRejectedValue({ code: 'failed', message: 'raw os text' });
    show(true)?.click();
    await vi.waitFor(() => {
      expect(document.querySelector('[role="alert"]')?.textContent).toBe('System Settings did not open.');
    });
  });
});
