import type { Capabilities } from './capabilities';

/** The answers of a client running in a browser: none of the desktop-only features (CLI-001, UPD-001). */
export const browserCapabilities: Capabilities = {
  keyStore: false,
  webViewTabs: false,
  popOut: false,
  updates: false,
  processLog: false,
  thisMachine: false,
  systemSettings: false,
};
