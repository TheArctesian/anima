import { desktopCapabilities } from '$lib/desktop/capabilities';
import { browserCapabilities } from './browser';
import type { Capabilities } from './capabilities';

export type { Capabilities } from './capabilities';

/**
 * What this client can do: the desktop shell's answers inside the desktop app, the browser's elsewhere (WP-42 D-4).
 * Read at the moment a control decides, as the checks it replaces were; the shell marks itself before any app script
 * runs, so every read gives the same answer.
 */
export function capabilities(): Capabilities {
  return desktopCapabilities() ?? browserCapabilities;
}
