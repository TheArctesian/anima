import type { UpdateSnapshot } from './schedule';

/** The update state in plain words for the Settings row (UPD-006). `time` writes the next check's clock time. */
export function describeUpdate({ state, nextCheckAt }: UpdateSnapshot, time: (at: Date) => string): string {
  switch (state.kind) {
    case 'idle':
      return 'Up to date';
    case 'checking':
      return 'Checking…';
    case 'downloading':
      return `Downloading Anima ${state.version}…`;
    case 'installed':
      return `Anima ${state.version} is installed — restart to update`;
    case 'failed':
      return nextCheckAt === null
        ? 'The last check failed.'
        : `The last check failed; the next check is at ${time(new Date(nextCheckAt))}.`;
  }
}

/** Whether "Check for updates" can run: not while a check or download runs, and not once an update is installed. */
export function canCheck({ state }: UpdateSnapshot): boolean {
  return state.kind === 'idle' || state.kind === 'failed';
}
