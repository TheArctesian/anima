import { desktopWebView } from '$lib/desktop/web-view/web-view';
import { capabilities } from '$lib/platform/platform';
import { WebViewError, type WebViewShell } from './shell';

const refuse = (): Promise<never> => Promise.reject(new WebViewError('failed'));

/**
 * No web view: a client without the desktop shell. It answers as the shell's commands do where there is no shell —
 * no pinning, and every command fails — so an `http` interface for the system browser opens a tab that says why,
 * and a tab shows its error page (WEB-010, WEB-011).
 */
const absentWebView: WebViewShell = {
  pinning: () => Promise.resolve(false),
  open: refuse,
  place: refuse,
  show: refuse,
  go: refuse,
  pin: refuse,
  popOut: refuse,
  close: refuse,
  openSystemBrowser: refuse,
  readCertificate: refuse,
  listen: refuse,
};

/** The web view shell of this client: the desktop shell's where it has web view tabs, none elsewhere (WP-42 D-4). */
export function webViewShell(): WebViewShell {
  return capabilities().webViewTabs ? desktopWebView : absentWebView;
}
