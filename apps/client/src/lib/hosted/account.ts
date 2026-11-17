import type { HubAccess } from './access';

/**
 * The Anima.ing cloud as the first run sees it (hosted-mode.md Section 4.1, HMD-022, HMD-023). The page calls only
 * this interface; `hostedParts()` in `choose.ts` picks the implementation. The implementation keeps the WorkOS tokens
 * itself (HMD-034, LOG-007); what the hub needs comes back sealed in a `HubAccess`.
 */
export interface HostedAccount {
  /** Starts the device sign-in: a short code to show and the page where the person signs in. */
  readonly start: () => Promise<Answer<SignInCode>>;
  /** Opens `page` in the system browser. */
  readonly openPage: (page: string) => Promise<Answer<void>>;
  /** Asks once whether the person signed in with `code`. */
  readonly poll: (code: SignInCode) => Promise<Answer<SignInPoll>>;
  /** Asks the cloud for the hub of the account; the cloud starts the make or wake job when it is not ready. */
  readonly hub: () => Promise<Answer<HubState>>;
  /**
   * One call for everything this computer needs to reach and use the ready hub: what the link join takes and the
   * credential of this device. The app joins with it silently (`join.ts`); the person never sees it (Section 4.3).
   */
  readonly access: () => Promise<Answer<HubAccess>>;
}

/** The code of one device sign-in. */
export interface SignInCode {
  /** The short code that the browser page also shows, for example `WDJB-MJHT`. */
  readonly code: string;
  /** The sign-in page, with the code filled in. */
  readonly page: string;
  /** Seconds between two polls. */
  readonly interval: number;
}

export type SignInPoll = 'waiting' | 'signed-in';

/** The hub of the account: ready, or a job of the cloud that makes a new hub or wakes a sleeping one (HMD-025). */
export type HubState = 'ready' | 'making' | 'waking';

/**
 * Why a step stopped. `cancelled`: the person cancelled in the browser. `expired`: the code ran out before the
 * sign-in. `offline`: the cloud did not answer. `refused`: the cloud or the hub said no.
 */
export type Problem = 'cancelled' | 'expired' | 'offline' | 'refused';

export type Answer<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly problem: Problem };

export function answer<T>(value: T): Answer<T> {
  return { ok: true, value };
}

export function refusal<T>(problem: Problem): Answer<T> {
  return { ok: false, problem };
}
