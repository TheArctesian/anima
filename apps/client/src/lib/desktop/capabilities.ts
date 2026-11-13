import { isTauri } from '@tauri-apps/api/core';
import type { Capabilities } from '$lib/platform/capabilities';
import { isMac } from '$lib/platform/mac';

/**
 * What the desktop shell can do, or `null` outside it (WP-42 D-4). The shell has the key store, web view tabs, the
 * updater, the process log, the builder (BLD-026) and the command line tool's link or launcher (INS-004), and reads
 * this computer; System Settings at Sharing exists on a Mac only (R-059, R-079), and only the Mac window draws under
 * its title bar. It has no pop-out window command yet, so `popOut` is no (RA-002, WP-14 H-2).
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
    systemSettings: isMac(userAgent),
    builds: true,
    mcpPermissions: true,
    commandLine: true,
    overlayTitleBar: isMac(userAgent),
  };
}
