import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Hub } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { readThisMachine, type ThisMachine } from '../this-machine';

/** Why the shell did not install the daemon: its error codes (MET-019, D-10 d). Any other answer is `failed`. */
export type DaemonShellProblem =
  | 'caller'
  | 'unavailable'
  | 'not-bundled'
  | 'busy'
  | 'code'
  | 'origin'
  | 'no-home'
  | 'untrusted'
  | 'copy'
  | 'enrol'
  | 'install'
  | 'timed-out'
  | 'failed';

const MESSAGES: Record<DaemonShellProblem, string> = {
  caller: 'Only the main window can install the daemon.',
  unavailable: 'The daemon cannot be installed on this system from the app.',
  'not-bundled': 'This build carries no daemon. Use the install command.',
  busy: 'An install is already running.',
  code: 'The hub made a code in a form the daemon does not accept.',
  origin: 'The hub address is not a plain https origin on port 443.',
  'no-home': 'The app could not find your home folder.',
  untrusted:
    '~/.local/bin, or a folder above it, can be changed by other users (through its group, an ACL or as others). Run `chmod g-w,o-w` on it, and `setfacl -b` for an ACL, then try again.',
  copy: 'The daemon could not be copied to ~/.local/bin.',
  enrol: 'The daemon could not enrol. Retry to make a new code.',
  install: 'The daemon enrolled but did not start. Run `anima-daemon install` in a terminal to see why.',
  'timed-out': 'The daemon did not finish in time.',
  failed: 'The install did not finish.',
};

const PROBLEMS = new Set<string>(Object.keys(MESSAGES));

export type DaemonShellResult = { readonly ok: true } | { readonly ok: false; readonly problem: DaemonShellProblem; readonly message: string };

/** The shell's error code; the shell's own text is never kept. */
function problemOf(reason: unknown): DaemonShellProblem {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return typeof code === 'string' && PROBLEMS.has(code) ? (code as DaemonShellProblem) : 'failed';
}

/** Whether this build of the desktop app carries the daemon; false outside it or when the shell does not answer. */
export async function daemonBundled(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    return (await invoke('daemon_bundled')) === true;
  } catch {
    return false;
  }
}

/**
 * Has the shell install the daemon on this computer for this user, enrolled with `code` at `origin` (MET-019). The code goes
 * to the shell and nowhere else; no answer and no message holds it (LOG-007).
 */
export async function installDaemon(code: string, origin: string): Promise<DaemonShellResult> {
  if (!isTauri()) return { ok: false, problem: 'unavailable', message: MESSAGES.unavailable };
  try {
    await invoke('daemon_install', { code, origin });
    return { ok: true };
  } catch (reason) {
    const problem = problemOf(reason);
    return { ok: false, problem, message: MESSAGES[problem] };
  }
}

/** What the Daemon panel needs from the desktop shell to install on the computer the app runs on. */
export interface ThisComputerShell {
  readonly thisMachine: () => Promise<ThisMachine | null>;
  readonly bundled: () => Promise<boolean>;
  readonly install: (code: string, origin: string) => Promise<DaemonShellResult>;
}

/** The desktop shell's calls for the computer `hub` sees, or `null` outside the desktop app (WP-42 D-4). */
export function thisComputerShell(hub: Hub): ThisComputerShell | null {
  if (!capabilities().thisMachine) return null;
  return { thisMachine: () => readThisMachine(hub), bundled: daemonBundled, install: installDaemon };
}
