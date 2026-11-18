import type { DeviceLinkStatus, LinkResult } from '$lib/desktop/link/shell';

/** The steps of a join, in order, as frame 2 of docs/mockup/mobile/index.html shows them (MOB-033). */
export const STEPS = [
  { id: 'invite', title: 'Invite read' },
  { id: 'link', title: 'Link started' },
  { id: 'hub', title: 'Waiting for hub' },
  { id: 'sign-in', title: 'Sign in' },
] as const;

export type StepId = (typeof STEPS)[number]['id'];

/** The mark of a step: ended, running now, failed, or not reached. */
export type StepState = 'done' | 'now' | 'failed' | 'wait';

/** The state of each step when `current` runs (or failed, with `failed`). Every step before it has ended. */
export function stepStates(current: StepId, failed: boolean): Readonly<Record<StepId, StepState>> {
  const at = STEPS.findIndex((step) => step.id === current);
  const entries = STEPS.map((step, index): [StepId, StepState] => {
    if (index < at) return [step.id, 'done'];
    if (index > at) return [step.id, 'wait'];
    return [step.id, failed ? 'failed' : 'now'];
  });
  return Object.fromEntries(entries) as Record<StepId, StepState>;
}

/** The link reaches the hub: it is up and has a path to it. A status with no path says only "up", which is enough. */
export function reachesHub(status: DeviceLinkStatus): boolean {
  return status.state === 'up' && status.path !== 'none';
}

/** What the wait needs; injected, so that tests need no shell and no clock. */
export interface Waiting {
  readonly read: () => Promise<LinkResult<DeviceLinkStatus>>;
  readonly pause: (ms: number) => Promise<void>;
  /** Ends the wait early, for Cancel. */
  readonly stopped: () => boolean;
}

export const HUB_SILENT = 'The hub did not answer. Check that it is on, then retry.';
const POLL_MS = 1000;
const TRIES = 60;

/** Waits up to a minute for the link to reach the hub: `null` when it does or the wait was stopped, else why not. */
export async function waitForHub(waiting: Waiting): Promise<string | null> {
  for (let tried = 0; tried < TRIES; tried += 1) {
    if (waiting.stopped()) return null;
    const status = await waiting.read();
    if (status.ok && reachesHub(status.value)) return null;
    await waiting.pause(POLL_MS);
  }
  return waiting.stopped() ? null : HUB_SILENT;
}
