import { capabilities } from '$lib/platform/platform';
import { webViewShell } from '$lib/web-interfaces/shell/client-shell';

/**
 * Where a one-time link goes. It is made inside the click, before the hub answers (a browser lets a click open a tab,
 * not a later callback). `open` hands the link on and keeps nothing; false means the browser did not show it.
 */
export interface LinkOpening {
  open(link: string): Promise<boolean>;
  /** Drops the opening, for example a blank tab, when no link comes. */
  close(): void;
}

/** The desktop: the system browser through the shell's command. */
export function systemBrowserOpening(openSystemBrowser: (url: string) => Promise<void>): LinkOpening {
  return {
    open: async (link) => {
      try {
        await openSystemBrowser(link);
        return true;
      } catch {
        return false;
      }
    },
    close: () => undefined,
  };
}

/** The web: a blank tab opened now, with no way back to this page, that gets its location later. */
export function blankTabOpening(openTab: () => Window | null): LinkOpening {
  const tab = openTab();
  if (tab !== null) tab.opener = null;
  return {
    open: (link) => {
      if (tab === null || tab.closed) return Promise.resolve(false);
      tab.location.href = link;
      return Promise.resolve(true);
    },
    close: () => {
      tab?.close();
    },
  };
}

/** The opening of this client: call it inside the click handler. */
export function linkOpening(): LinkOpening {
  if (capabilities().webViewTabs) return systemBrowserOpening((url) => webViewShell().openSystemBrowser(url));
  return blankTabOpening(() => window.open('about:blank', '_blank'));
}
