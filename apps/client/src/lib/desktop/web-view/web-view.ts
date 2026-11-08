import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { webUrlFrom, webViewEventFrom } from '$lib/web-interfaces/shell/events';
import { certificateFrom, codeOf, WebViewError, type WebViewShell } from '$lib/web-interfaces/shell/shell';

async function command(name: string, args: Record<string, unknown>): Promise<unknown> {
  try {
    return await invoke(name, args);
  } catch (reason) {
    throw new WebViewError(codeOf(reason));
  }
}

async function pinning(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    // Anything but `true` keeps the system browser, which never trusts a certificate the operator did not see.
    return (await invoke('web_view_pinning')) === true;
  } catch {
    return false;
  }
}

/**
 * The web view commands of the desktop shell. The event is heard on the main web view only: the shell emits it to
 * `main`, and device web views have no IPC to emit with (D-3).
 */
export const desktopWebView: WebViewShell = {
  pinning,
  open: async ({ tabId, deviceId, url, pin, bounds }) => {
    await command('web_view_open', { tabId, deviceId, url, pin, bounds });
  },
  place: async (tabId, bounds) => {
    await command('web_view_place', { tabId, bounds });
  },
  show: async (tabId, shown) => {
    await command('web_view_show', { tabId, shown });
  },
  go: async (tabId, to) => {
    await command('web_view_go', { tabId, to });
  },
  pin: async (tabId, pin) => {
    await command('web_view_pin', { tabId, pin });
  },
  popOut: async (tabId) => {
    await command('web_view_pop_out', { tabId });
  },
  close: async (tabId) => {
    await command('web_view_close', { tabId });
  },
  openSystemBrowser: async (url) => {
    // The shell checks the scheme too; this keeps a wrong URL from ever reaching it.
    if (webUrlFrom(url) === null) throw new WebViewError('invalid_url');
    await command('system_browser_open', { url });
  },
  readCertificate: async (url) => {
    if (webUrlFrom(url) === null || new URL(url).protocol !== 'https:') throw new WebViewError('invalid_url');
    return certificateFrom(await command('web_certificate_read', { url }));
  },
  listen: (onevent) =>
    listen<unknown>(
      'web-view',
      ({ event, payload }) => {
        if (event !== 'web-view') return;
        const parsed = webViewEventFrom(payload);
        if (parsed !== null) onevent(parsed);
      },
      { target: { kind: 'WebviewWindow', label: 'main' } },
    ),
};
