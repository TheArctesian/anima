const STORAGE_KEY = 'anima.hostedHub';

/**
 * Marks `origin` as the address of a hub hosted by Anima.ing, where the account is the only way in (R-129, HMD-037): a
 * device that must sign in again goes to the first-run choices, whose primary choice is "Sign in", never to the hub
 * password page. The mark is not a secret.
 */
export function markHosted(storage: Storage, origin: string): void {
  storage.setItem(STORAGE_KEY, origin);
}

/** Whether `origin` is the address of a hosted hub; `false` when the storage refuses to be read (privacy modes). */
export function isHostedHub(storage: Storage, origin: string): boolean {
  try {
    return storage.getItem(STORAGE_KEY) === origin;
  } catch {
    return false;
  }
}

/** The storage of the browser, or `null` where there is none or it refuses to be read. */
function browserStore(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/**
 * Whether the hub at `origin` is hosted by Anima.ing. The one place where the client decides it (R-222): such a hub is
 * reached through the link and has no VPN (DEF-005), so no screen names a VPN for it.
 */
export function hubIsHosted(origin: string): boolean {
  const storage = browserStore();
  return storage !== null && isHostedHub(storage, origin);
}

/**
 * The address of the hosted hubs in a browser (HMD-059). Until 2026-11-20 it was the default address of a hub of
 * one's own, so an application of that time may still hold it as the address of its hub (HM-T31, R-269).
 */
export const HOSTED_BROWSER_ORIGIN = 'https://hub.anima.ing';

/** The page of the Anima.ing account: its email, its sign-in methods and its browser sessions (HMD-062). */
export const ACCOUNT_PAGE = 'https://cloud.anima.ing/account';

/** What the application says when the hub of its settings is that address: the hub of before is not there. */
export const HOSTED_ADDRESS_MOVED =
  'hub.anima.ing is now the address of the hosted Anima.ing hubs in a browser. Sign in with your Anima.ing account, or give the address of your own hub.';

/**
 * Whether a client whose page came from `pageOrigin` holds the address of the hosted hubs as the address of its own
 * hub. The application reaches a hosted hub through its link, at a port of this computer, never at this address, so an
 * application that asks there has the hub of before in its settings. A browser page that this address served is at its
 * own hub.
 */
export function holdsMovedAddress(pageOrigin: string | null, origin: string): boolean {
  return origin === HOSTED_BROWSER_ORIGIN && pageOrigin !== origin;
}

/** `holdsMovedAddress` for the page of this client; where there is no page (a test with no browser), no. */
export function holdsMovedAddressHere(origin: string): boolean {
  return typeof location !== 'undefined' && holdsMovedAddress(location.origin, origin);
}

/** What the operator can check when the hub does not answer, with no full stop (NET-004, R-222). */
export function networkAdvice(hosted: boolean): string {
  return hosted ? "Check this computer's network" : 'Check the VPN client';
}
