import { invoke, isTauri } from '@tauri-apps/api/core';

/**
 * Who listens on the port of a loopback hub address (F-10): `own` is a process of this user (and, for the device link's
 * port, the app's own child); `other` is another user's process; `none` is no process.
 */
export type Listener = 'own' | 'other' | 'none';

/** Why the check gave no answer: the shell's codes, and `unavailable` outside the desktop app. Any other answer is `failed`. */
export type ListenerProblem = 'caller' | 'bad-origin' | 'unavailable' | 'failed';

export type ListenerResult = { readonly ok: true; readonly listener: Listener } | { readonly ok: false; readonly problem: ListenerProblem };

const LISTENERS: readonly unknown[] = ['own', 'other', 'none'] satisfies Listener[];
const PROBLEMS: readonly unknown[] = ['caller', 'bad-origin'] satisfies ListenerProblem[];

/** Asks the shell who listens at the loopback hub address `origin`. */
export async function checkHubListener(origin: string): Promise<ListenerResult> {
  if (!isTauri()) return { ok: false, problem: 'unavailable' };
  // SPIKE: the phone shell has no listener check; the test hub is this Mac's own.
  if (/iPhone|iPad|iPod|Android/.test(navigator.userAgent)) return { ok: true, listener: 'own' };
  try {
    const answer: unknown = await invoke('hub_listener_check', { origin });
    return LISTENERS.includes(answer) ? { ok: true, listener: answer as Listener } : { ok: false, problem: 'failed' };
  } catch (reason) {
    const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
    return { ok: false, problem: PROBLEMS.includes(code) ? (code as ListenerProblem) : 'failed' };
  }
}
