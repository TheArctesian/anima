// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import RemoteLoginButton from './RemoteLoginButton.svelte';
import { openRemoteLoginSettings, remoteLoginSettingsAvailable, SystemSettingsError } from './systemSettings';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15';
const LINUX = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15';
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

describe('remoteLoginSettingsAvailable', () => {
  it('R-059: only the desktop app on a Mac offers the Remote Login settings', () => {
    expect(remoteLoginSettingsAvailable(MAC)).toBe(true);
    expect(remoteLoginSettingsAvailable(LINUX)).toBe(false);
    shell.isTauri.mockReturnValue(false);
    expect(remoteLoginSettingsAvailable(MAC)).toBe(false);
  });
});

describe('openRemoteLoginSettings', () => {
  it('R-059: invokes the shell command with no arguments, so the web view names no URL', async () => {
    shell.invoke.mockResolvedValue(null);
    await openRemoteLoginSettings();
    expect(shell.invoke).toHaveBeenCalledWith('remote_login_settings_open');
  });

  it('R-059: shell codes become typed errors without the shell message', async () => {
    shell.invoke.mockRejectedValue({ code: 'unavailable', message: 'raw os text' });
    const failure = await openRemoteLoginSettings().catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(SystemSettingsError);
    expect(failure).toMatchObject({ problem: 'unavailable' });
    expect(String(failure)).not.toContain('raw os text');
    shell.invoke.mockRejectedValue('command not found');
    await expect(openRemoteLoginSettings()).rejects.toMatchObject({ problem: 'failed' });
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
    expect(shell.invoke).toHaveBeenCalledWith('remote_login_settings_open');
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
