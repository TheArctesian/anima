/** How far a session start has come, from facts the client observes. */
export interface Progress {
  /** `startSession` answered with a ticket. */
  readonly ticketed: boolean;
  /** The hub sent a Wake-on-LAN packet for this start (`SessionTicket.wakeSent`, K4 §1); false until it answers. */
  readonly waking: boolean;
  /** The WebSocket upgrade succeeded (`TunnelState.opened`). */
  readonly opened: boolean;
  /** The socket is open: the terminal can be used. */
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
 * - gateway: the hub upgrades the WebSocket, which it does only after it signed in to the device (K4 §8.2), so the
 *   hub holds the connection;
 * - device: the socket is open; the hub's SSH connection to the device is the one the hub just opened;
 * - ready: the session can be used. No later signal exists, so it ends with `device`.
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

/** The line under the steps for a start that sent a wake packet (K4 §1): its text and mark. */
export interface WakeLine {
  readonly label: string;
  readonly mark: StepMark;
}

/**
 * The wake line, or `undefined` when no packet was sent. The hub opens the socket once the device answers, so the
 * wait ends with `opened`. `deviceName` is text only.
 */
export function wakeLine(progress: Progress, deviceName: string): WakeLine | undefined {
  if (!progress.waking) return undefined;
  const mark: StepMark = progress.opened ? 'done' : progress.stopped ? 'failed' : 'active';
  return { label: `Waking ${deviceName}`, mark };
}
