import type { DaemonHere } from './here';
import { describeDaemonProblem } from './shell';

/**
 * Where the install of the daemon on the computer the app runs on stands (HMD-053): nothing started (also in a
 * browser, where no install runs), it runs, the daemon is there, or it ended with a reason in words for a person.
 * It is known only on the computer that runs the app and only for that computer; no hub field holds it.
 */
export type DaemonHereState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'installing' }
  | { readonly kind: 'installed' }
  | { readonly kind: 'failed'; readonly reason: string };

const NOT_FOUND = 'The application did not find this computer among the devices of the hub.';
const NO_CODE = 'The hub made no enrolment code.';

/** The state after an install that ended with `did`. */
export function stateAfter(did: DaemonHere): DaemonHereState {
  switch (did.kind) {
    case 'installed':
    case 'present':
      return { kind: 'installed' };
    case 'not-ready':
      return { kind: 'failed', reason: NOT_FOUND };
    case 'failed':
      return { kind: 'failed', reason: did.step === 'enrol' ? NO_CODE : describeDaemonProblem(did.step) };
  }
}

/** One sentence for a page of a hosted hub that waits for the daemon of this computer (AIS-020, HMD-057). */
export function daemonHereLine(state: DaemonHereState): string {
  switch (state.kind) {
    case 'idle':
      return 'Open the Anima.ing application on a computer and sign in: it installs the daemon there.';
    case 'installing':
      return 'The Anima.ing application is installing the daemon on this computer. Counts appear here after its first report.';
    case 'installed':
      return 'The daemon of this computer is installed. Counts appear here some minutes after you use Claude Code or Codex on it.';
    case 'failed':
      return `The daemon was not installed on this computer. ${state.reason} Quit the application and open it again to try once more.`;
  }
}

/** The state of this start of the app. Pages read `daemonHere.current`; only the install writes it. */
export const daemonHere = $state<{ current: DaemonHereState }>({ current: { kind: 'idle' } });
