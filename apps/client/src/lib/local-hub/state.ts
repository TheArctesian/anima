import type { BlockedReason, LocalHubResult, LocalHubStatus } from '$lib/desktop/local-hub/shell';

/** Where an install or a start has got to: still on its way, or settled in one of three ends (FRN-002). */
export type Progress = 'waiting' | 'ready' | 'blocked' | 'failed';

/** The progress the status shows. A hub that is not installed, or stopped, is still on its way until it reports. */
export function progressOf(status: LocalHubStatus): Progress {
  switch (status.state) {
    case 'ready':
    case 'blocked':
    case 'failed':
      return status.state;
    default:
      return 'waiting';
  }
}

/** Where the hub root is: what the operator needs to put `vault.key` back (`docs/setup/local-hub.md` Section 1). */
export function hubRoot(mac: boolean): string {
  return mac ? '~/Library/Application Support/ing.anima.desktop/hub' : '~/.local/share/anima/hub';
}

const BLOCKED: Readonly<Record<BlockedReason, (mac: boolean) => string>> = {
  // Never a new key: it cannot open what the old key sealed (2.8, OR-S6-1).
  'key-missing': (mac) =>
    `Blocked: the hub key vault.key is missing. Put your copy back in ${hubRoot(mac)}, then start the hub.`,
  'database-version': () => 'Blocked: the database is from another PostgreSQL version.',
  'database-in-use': () => 'Blocked: another program runs the hub database. Stop it, then start the hub.',
};

const LINK: Readonly<Record<NonNullable<LocalHubStatus['link']>['state'], string>> = {
  starting: 'Link starting.',
  up: 'Link up: devices can join.',
  down: 'Link down, retrying. This computer still reaches the hub.',
};

/** One true sentence for the hub's state (FRN-006, LHB-010). */
export function stateText(status: LocalHubStatus, mac: boolean): string {
  if (!status.installed) return 'Not installed.';
  switch (status.state) {
    case undefined:
    case 'starting':
      return 'Starting. The first start can take a few minutes.';
    case 'ready':
      return status.recovered === true ? 'Ready. Recovered after an unclean stop; nothing was lost.' : 'Ready.';
    case 'blocked':
      return status.reason === undefined ? 'Blocked.' : BLOCKED[status.reason](mac);
    case 'stopping':
      return 'Stopping.';
    case 'stopped':
      return 'Stopped. Starts at the next sign-in to this computer.';
    case 'failed':
      return `Failed: ${status.cause ?? 'a program'} ended. Restarting in 10 seconds.`;
  }
}

/** The link member's sentence, or `null` when the status carries none (F-6). */
export function linkText(status: LocalHubStatus): string | null {
  return status.link === undefined ? null : LINK[status.link.state];
}

/** What polling needs; each part is injected, so that tests need no shell or timer (CQ-021). */
export interface Poll {
  readonly read: () => Promise<LocalHubResult<LocalHubStatus>>;
  readonly wait: (ms: number) => Promise<void>;
  /** Each status read, for the screen. */
  readonly seen: (status: LocalHubStatus) => void;
  /** True once the screen has gone: polling stops. */
  readonly stopped: () => boolean;
}

/** One read every second. */
export const POLL_MS = 1000;

/**
 * Reads the status every second until it settles (ready or blocked), the shell refuses or the screen goes. A failed
 * hub starts again by itself in 10 seconds, so the poll goes on through it and shows each status.
 * Returns the last answer, or `null` when stopped.
 */
export async function pollUntilSettled(poll: Poll): Promise<LocalHubResult<LocalHubStatus> | null> {
  while (!poll.stopped()) {
    const result = await poll.read();
    if (poll.stopped()) return null;
    if (!result.ok) return result;
    poll.seen(result.value);
    const progress = progressOf(result.value);
    if (progress === 'ready' || progress === 'blocked') return result;
    await poll.wait(POLL_MS);
  }
  return null;
}
