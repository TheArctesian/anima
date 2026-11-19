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
 * Whether the hub at `origin` is hosted by Anima.ing. The one place where the client decides it (R-211): such a hub is
 * reached through the link and has no VPN (DEF-005), so no screen names a VPN for it.
 */
export function hubIsHosted(origin: string): boolean {
  const storage = browserStore();
  return storage !== null && isHostedHub(storage, origin);
}

/** What the operator can check when the hub does not answer, with no full stop (NET-004, R-211). */
export function networkAdvice(hosted: boolean): string {
  return hosted ? "Check this computer's network" : 'Check the VPN client';
}
