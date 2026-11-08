import { isTauri } from '@tauri-apps/api/core';
import type { Capabilities } from '$lib/platform/capabilities';

const MAC = /Macintosh|Mac OS X/;

/**
 * What the desktop shell can do, or `null` outside it (WP-42 D-4). The shell has the key store, web view tabs, the
 * updater and the process log, and reads this computer; System Settings at Sharing exists on a Mac only (R-059, R-079).
 * It has no pop-out window command yet, so `popOut` is no (RA-002, WP-14 H-2).
 */
export function desktopCapabilities(userAgent: string = navigator.userAgent): Capabilities | null {
  if (!isTauri()) return null;
  return {
    keyStore: true,
    webViewTabs: true,
    popOut: false,
    updates: true,
    processLog: true,
    thisMachine: true,
    systemSettings: MAC.test(userAgent),
  };
}
