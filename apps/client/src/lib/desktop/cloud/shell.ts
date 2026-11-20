import { invoke, isTauri } from '@tauri-apps/api/core';
import { sealAccess } from '$lib/hosted/access';
import { answer, refusal, type Answer, type HostedAccount, type HubState, type Problem, type SignInCode } from '$lib/hosted/account';

/** The shell's error codes (`apps/desktop/src-tauri/src/cloud/error.rs`) as the problems the first run shows. */
const PROBLEMS: Readonly<Record<string, Problem>> = {
  unconfigured: 'unconfigured',
  cancelled: 'cancelled',
  expired: 'expired',
  offline: 'offline',
  'not-ready': 'offline',
  'payment-failed': 'payment',
  unauthenticated: 'signed-out',
};

function problemOf(reason: unknown): Problem {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return (typeof code === 'string' ? PROBLEMS[code] : undefined) ?? 'refused';
}

/** Runs a shell command; its answer is checked by `shape`, and any failure is a problem, never its text (LOG-007). */
async function command<T>(name: string, shape: (value: unknown) => T | undefined, args?: Record<string, unknown>): Promise<Answer<T>> {
  if (!isTauri()) return refusal('unconfigured');
  try {
    const value = shape(await invoke(name, args));
    return value === undefined ? refusal('refused') : answer(value);
  } catch (reason) {
    return refusal(problemOf(reason));
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;

function signInCodeOf(value: unknown): SignInCode | undefined {
  if (!isRecord(value) || typeof value.code !== 'string' || typeof value.page !== 'string' || typeof value.interval !== 'number') return undefined;
  return { code: value.code, page: value.page, interval: value.interval };
}

const HUB_STATES: readonly unknown[] = ['ready', 'making', 'waking'] satisfies HubState[];

/**
 * The Anima.ing cloud through the shell (HMD-043): the WorkOS device sign-in, the hub of the account and this device's
 * access to it. The shell does the calls, so the WorkOS tokens stay in the shell and its key store (HMD-034); this side
 * sees the short code, the page, the hub state, and the invite and the claim, which go straight to the join.
 *
 * With `inApp` (the iPhone app, mobile plan M3-5) a new sign-in has no code: the shell shows the sign-in page inside
 * the app and answers when the person signed in or closed it.
 */
export function cloudAccount(inApp = false): HostedAccount {
  return {
    start: async () => {
      // A session that still holds needs no browser step: the account signs in again silently.
      const resumed = await command('cloud_sign_in_resume', (value) => (typeof value === 'boolean' ? value : undefined));
      if (resumed.ok && resumed.value) return answer('signed-in');
      if (!resumed.ok && resumed.problem === 'unconfigured') return resumed;
      if (!inApp) return command('cloud_sign_in_start', signInCodeOf);
      const signedIn = await command('cloud_sign_in_session', (value) => (value === true ? value : undefined));
      return signedIn.ok ? answer('signed-in') : signedIn;
    },
    openPage: async (page) => {
      try {
        await invoke('system_browser_open', { url: page });
        return answer(undefined);
      } catch {
        return refusal('refused');
      }
    },
    poll: async () => {
      const polled = await command('cloud_sign_in_poll', (value) => (typeof value === 'boolean' ? value : undefined));
      return polled.ok ? answer(polled.value ? 'signed-in' : 'waiting') : polled;
    },
    hub: () => command('cloud_hub', (value) => (HUB_STATES.includes(value) ? (value as HubState) : undefined)),
    access: () =>
      command('cloud_hub_access', (value) =>
        isRecord(value) && (typeof value.invite === 'string' || value.invite === null) && typeof value.claim === 'string'
          ? sealAccess({ invite: value.invite, claim: value.claim })
          : undefined,
      ),
  };
}
