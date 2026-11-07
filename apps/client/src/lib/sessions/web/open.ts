import type { OpenIn, Status } from '$lib/hub/generated/graphql';
import type { WebViewShell } from '$lib/desktop/webView/webView';
import type { Sessions } from '../sessions.svelte';
import type { Origin, WebTarget } from '../tabs';

/** The Web button that was pressed: the device, its interface and the control the tab grows from (WEB-003, MOT-005). */
export interface WebOpening {
  readonly device: { readonly id: string; readonly name: string; readonly status: Status };
  readonly webInterface: WebTarget & { readonly openIn: OpenIn };
  readonly origin: Origin | null;
}

/** An `http` interface that opens in the system browser: nothing there needs a tab, since there is no pin to read (WEB-011). */
function belongsToBrowser({ webInterface }: WebOpening): boolean {
  return webInterface.openIn === 'SYSTEM_BROWSER' && URL.canParse(webInterface.url) && new URL(webInterface.url).protocol === 'http:';
}

/**
 * Opens the interface as its setting says (WEB-003, WEB-011): `ANIMA_TAB` in a web tab; `SYSTEM_BROWSER` over `http` in
 * the system browser; over `https` in a web tab too, which reads and compares the pinned fingerprint before it hands
 * the URL to the browser (WEB-008). When the browser cannot be opened, the tab opens instead and shows why.
 */
export async function openWebInterface(sessions: Pick<Sessions, 'openWeb'>, shell: Pick<WebViewShell, 'openSystemBrowser'>, opening: WebOpening): Promise<void> {
  if (belongsToBrowser(opening)) {
    try {
      await shell.openSystemBrowser(opening.webInterface.url);
      return;
    } catch {
      // The tab below says what is wrong; the shell's own message is not shown (LOG-007).
    }
  }
  const { device, webInterface, origin } = opening;
  const { id, deviceId, url, label, pinnedFingerprintSha256 } = webInterface;
  sessions.openWeb({ deviceId: device.id, deviceName: device.name, status: device.status, web: { id, deviceId, url, label, pinnedFingerprintSha256 }, origin });
}
