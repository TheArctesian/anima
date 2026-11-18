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
