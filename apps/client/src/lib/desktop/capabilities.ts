import { isTauri } from '@tauri-apps/api/core';
import { browserCapabilities } from '$lib/platform/browser';
import type { Capabilities } from '$lib/platform/capabilities';
import { isMac } from '$lib/platform/mac';
import { isMobile, type Agent } from '$lib/platform/mobile';

/**
 * What the mobile shell can do (MOB-002, MOB-004): it keeps the access token, runs the device link inside the app
 * (MOB-031, mobile plan M3-1) and signs in to the Anima.ing account (mobile plan M3-4), and it has no desktop-only
 * feature. A phone has no updater (UPD-001), runs no hub and no build, and reads no computer. Its copy of recent data is
 * the mobile plan's (IOS-003), so it keeps none here.
 */
const MOBILE_SHELL: Capabilities = { ...browserCapabilities, keyStore: true, deviceLink: true, cloudAccount: true };

/**
 * What the shell can do, or `null` outside it (WP-42 D-4). On a phone or a tablet the answers are the mobile shell's.
 * The desktop shell has the key store, web view tabs, the
 * updater, the process log, the builder (BLD-026), the command line tool's link or launcher (INS-004) and the
 * stay-awake control (AWK-004), the local hub and the device link (FRN-002, FRN-005; the app runs on macOS and Linux), and reads
 * this computer; System Settings at Sharing exists on a Mac only (R-059, R-079), and only the Mac window draws under
 * its title bar. It keeps the copy of recent hub answers (FND-007). It has no pop-out window command yet, so `popOut` is no (RA-002, WP-14 H-2).
 */
export function shellCapabilities(agent: Agent = navigator): Capabilities | null {
  if (!isTauri()) return null;
  if (isMobile(agent)) return MOBILE_SHELL;
  const { userAgent } = agent;
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
    stayAwake: true,
  localHub: true,
    deviceLink: true,
    cloudAccount: true,
    overlayTitleBar: isMac(userAgent),
    recentCopy: true,
  };
}
