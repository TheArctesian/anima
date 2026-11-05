/** How far a session start has come, from facts the client observes. */
export interface Progress {
  /** `startSession` answered with a ticket. */
  readonly ticketed: boolean;
  /** The hub's first instruction arrived (`TunnelState.opened`). */
  readonly opened: boolean;
  /** The gateway's first frame (`sync`) arrived: the client turned CONNECTED. */
  readonly connected: boolean;
  /** The start stopped: refused, or the tunnel closed before it was connected. */
  readonly stopped: boolean;
}

export type StepName = 'request' | 'gateway' | 'device' | 'ready';

/** `waiting` has not begun, `active` is under way, `done` ended well, `failed` is where the start stopped. */
export type StepMark = 'waiting' | 'active' | 'done' | 'failed';

export interface Step {
  readonly name: StepName;
  readonly label: string;
  readonly mark: StepMark;
}

/**
 * The connection steps (Section 5.7.3) and the observed fact that ends each one:
 * - request: the hub issues a ticket (`startSession` answers);
 * - gateway: the hub sends its first tunnel instruction, which it does only after its handshake with guacd ended
 *   with `ready` (K4 §3, §4), so the gateway has taken the connection;
 * - device: the gateway's first frame (`sync`) arrives, which it sends only once it is connected to the device;
 * - ready: the session can be used. No later signal exists, so it ends on the same frame as `device`.
 */
const STEPS: readonly { readonly name: StepName; readonly label: string; readonly ended: (p: Progress) => boolean }[] = [
  { name: 'request', label: 'Request', ended: (p) => p.ticketed },
  { name: 'gateway', label: 'Gateway', ended: (p) => p.opened },
  { name: 'device', label: 'Device', ended: (p) => p.connected },
  { name: 'ready', label: 'Ready', ended: (p) => p.connected },
];

/** Each step's mark: ended steps are done, the first other one is active (or failed once stopped), the rest wait. */
export function connectionSteps(progress: Progress): readonly Step[] {
  const current = STEPS.findIndex((step) => !step.ended(progress));
  return STEPS.map(({ name, label }, index) => {
    if (current === -1 || index < current) return { name, label, mark: 'done' };
    if (index > current) return { name, label, mark: 'waiting' };
    return { name, label, mark: progress.stopped ? 'failed' : 'active' };
  });
}
