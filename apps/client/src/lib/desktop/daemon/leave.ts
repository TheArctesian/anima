import { hubIsHosted } from '$lib/hosted/mark';
import type { Hub, HubResult } from '$lib/hub/hub';
import { metricsApi } from '$lib/metrics/api';
import { capabilities } from '$lib/platform/platform';
import { warnToProcessLog } from '../process-log';
import { deviceHere, findHereParts, type FindHereParts, type NotFound } from './here';
import { removeDaemon, type DaemonRemoval } from './shell';

/** Said when the daemon left this computer but the hub was not told: the hub still lists it (HMD-056). */
export const HUB_STILL_LISTS =
  'The daemon is removed from this computer, but the hub could not be reached and still lists it. Revoke the daemon on the page of this computer in Devices when the hub answers.';
/** Said first when the daemon is still on this computer; the reason of the shell follows. */
export const NOT_REMOVED = 'The daemon was not removed from this computer.';
/** Added to `NOT_REMOVED` when the hub was not told either. */
export const HUB_NOT_TOLD = 'The hub still lists it too.';

/** What the removal of the daemon of this computer needs; each part is injected, so that tests need no shell and no hub. */
export interface DaemonLeaveParts extends FindHereParts {
  /** Has the shell take the daemon off this computer. */
  readonly remove: () => Promise<DaemonRemoval>;
  /** Ends the daemon token of the device at the hub, with its unused codes. */
  readonly revoke: (deviceId: string) => Promise<HubResult<unknown>>;
  /** Takes one line for the log of the app: fixed text and fixed codes only. */
  readonly log?: (line: string) => void;
}

/** This computer is not a device of the hub: the hub lists no daemon of it, and nothing is revoked. */
const NOT_LISTED: ReadonlySet<NotFound> = new Set(['not-on-the-link', 'not-a-device-yet']);

/** Whether the hub lists no daemon of this computer any more: it revoked it, or this computer is not one of its devices. */
async function revokedAtHub(parts: DaemonLeaveParts, log: (line: string) => void): Promise<boolean> {
  const device = await deviceHere(parts);
  if (typeof device === 'string') {
    log(`the removal of the daemon: this computer was not found at the hub (${device})`);
    return NOT_LISTED.has(device);
  }
  const revoked = await parts.revoke(device.id);
  log(`the removal of the daemon: ${revoked.ok ? 'the hub revoked it' : `the hub did not revoke it (${revoked.error.code})`}`);
  return revoked.ok;
}

/**
 * Takes the daemon off the computer the app runs on and revokes it at the hub (HMD-056). The removal on this computer
 * runs first and whatever the hub does. Returns the sentence to show when something stays (on this computer, or the
 * listing at the hub); `null` when nothing stays.
 */
export async function leaveDaemonHere(parts: DaemonLeaveParts): Promise<string | null> {
  const log = parts.log ?? ((): void => undefined);
  const removed = await parts.remove();
  log(`the removal of the daemon: ${removed.ok ? (removed.wasHere ? 'removed from this computer' : 'no copy was here') : `not removed (${removed.problem})`}`);
  const told = await revokedAtHub(parts, log).catch(() => false);
  if (!removed.ok) return [NOT_REMOVED, removed.message, ...(told ? [] : [HUB_NOT_TOLD])].join(' ');
  return removed.wasHere && !told ? HUB_STILL_LISTS : null;
}

/**
 * The removal for a sign-out of `hub` in the desktop app: only for a hosted hub, where the app installed the daemon with
 * no question (HMD-053). It must run before the sign-out, while the token can still revoke at the hub.
 */
export async function leaveDaemonOnThisComputer(hub: Hub): Promise<string | null> {
  if (!capabilities().thisMachine || !hubIsHosted(hub.address.origin)) return null;
  return leaveDaemonHere({
    ...findHereParts(hub),
    remove: removeDaemon,
    revoke: (deviceId) => metricsApi(hub).revokeDaemonToken(deviceId),
    log: warnToProcessLog,
  });
}
