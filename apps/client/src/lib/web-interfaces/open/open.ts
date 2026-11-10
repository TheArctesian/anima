import type { OpenIn, OperatingSystem, Status } from '$lib/hub/generated/graphql';
import type { WebViewShell } from '$lib/web-interfaces/shell/shell';
import type { Sessions } from '$lib/sessions/sessions.svelte';
import { capabilities } from '$lib/platform/platform';
import type { Origin, WebTarget } from '$lib/sessions/tabs';

/** The Web button that was pressed: the device, its interface and the control the tab grows from (WEB-003, MOT-005). */
export interface WebOpening {
  readonly device: { readonly id: string; readonly name: string; readonly status: Status; readonly os?: OperatingSystem; readonly distribution?: string | null };
  readonly webInterface: WebTarget & { readonly openIn: OpenIn };
  readonly origin: Origin | null;
}

/** An `http` interface that opens in the system browser: nothing there needs a tab, since there is no pin to read (WEB-011). */
function belongsToBrowser({ webInterface }: WebOpening): boolean {
  return webInterface.openIn === 'SYSTEM_BROWSER' && URL.canParse(webInterface.url) && new URL(webInterface.url).protocol === 'http:';
}

/** A browser tab for `url` when it is an `http:` or `https:` address; any other scheme, or text that is no URL, opens nothing (WEB-011). */
function openInBrowserTab(url: string): void {
  const parsed = URL.parse(url);
  if (parsed === null || (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')) return;
  // `noopener` makes `window.open` answer null whether or not the tab opened, so null is not a failure. The URL is not logged (LOG-007).
  // The parsed address is opened, not the text: `https:evil` is checked as host `evil` and must open as such, not against the page.
  window.open(parsed.href, '_blank', 'noopener,noreferrer');
}

/**
 * In a browser there are no web view tabs and no Anima tab (WEB-011): every interface opens in a new browser tab, straight
 * to the device. The call is made before any `await`, inside the click, so a popup blocker sees the gesture. An address
 * that is not `http`/`https` opens nothing and says nothing: the card has no place for a message.
 * Otherwise, as its setting says (WEB-003, WEB-011): `ANIMA_TAB` in a web tab; `SYSTEM_BROWSER` over `http` in
 * the system browser; over `https` in a web tab too, which reads and compares the pinned fingerprint before it hands
 * the URL to the browser (WEB-008). When the browser cannot be opened, the tab opens instead and shows why.
 */
export async function openWebInterface(sessions: Pick<Sessions, 'openWeb'>, shell: Pick<WebViewShell, 'openSystemBrowser'>, opening: WebOpening): Promise<void> {
  if (!capabilities().webViewTabs) {
    openInBrowserTab(opening.webInterface.url);
    return;
  }
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
  sessions.openWeb({ deviceId: device.id, deviceName: device.name, ...(device.os === undefined ? {} : { os: device.os }), ...(device.distribution == null ? {} : { distribution: device.distribution }), status: device.status, web: { id, deviceId, url, label, pinnedFingerprintSha256 }, origin });
}
