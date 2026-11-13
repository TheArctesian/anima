import { browserCapabilities } from './browser';
import type { Capabilities } from './capabilities';

// Test support: the answers a test mocks `$lib/platform/platform` with, so it can show a control with and without
// the desktop shell.

/** The desktop app on a Mac, as `desktopCapabilities` answers there. */
export const DESKTOP_MAC: Capabilities = {
  keyStore: true,
  webViewTabs: true,
  popOut: false,
  updates: true,
  processLog: true,
  thisMachine: true,
  systemSettings: true,
  builds: true,
  mcpPermissions: true,
  overlayTitleBar: true,
};

/** A browser: every desktop-only feature off. */
export const BROWSER: Capabilities = browserCapabilities;
