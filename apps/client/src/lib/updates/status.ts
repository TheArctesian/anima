import type { FailureReason, UpdateSnapshot } from './schedule';

const NEXT = (nextCheckAt: number | null, time: (at: Date) => string): string =>
  nextCheckAt === null ? '' : ` The next check is at ${time(new Date(nextCheckAt))}.`;

/** What a failed attempt says, by reason. A Mac with no feed is expected, not broken (BLD-068). */
function describeFailure(reason: FailureReason, mac: boolean): string {
  switch (reason) {
    case 'no-feed':
      return mac
        ? 'Updates for the Mac are not published yet. Install a new version from its disk image.'
        : 'No update is published for this computer yet.';
    case 'signature':
      return 'An update was refused because its signature did not verify. Anima kept the version it has.';
    case 'other':
      return 'The last update attempt failed.';
  }
}

/**
 * The update state in plain words for the Settings row (UPD-006). `time` writes the next check's clock time; `mac` says
 * this computer is a Mac.
 */
export function describeUpdate({ state, nextCheckAt }: UpdateSnapshot, time: (at: Date) => string, mac = false): string {
  switch (state.kind) {
    case 'unchecked':
      return 'Not checked yet';
    case 'idle':
      return 'Up to date';
    case 'checking':
      return 'Checking…';
    case 'downloading':
      return `Downloading Anima ${state.version}…`;
    case 'installed':
      return `Anima ${state.version} is installed — restart to update`;
    case 'failed':
      return describeFailure(state.reason, mac) + NEXT(nextCheckAt, time);
  }
}

/** Whether "Check for updates" can run: not while a check or download runs, and not once an update is installed. */
export function canCheck({ state }: UpdateSnapshot): boolean {
  return state.kind === 'unchecked' || state.kind === 'idle' || state.kind === 'failed';
}
