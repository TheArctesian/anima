import type { Level, StayAwakeProblem, StayAwakeStatus } from '$lib/desktop/stay-awake/shell';

/** The three states in the order the control shows them (AWK-001). */
export const LEVELS: readonly { readonly value: Level; readonly label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'awake', label: 'Keep awake' },
  { value: 'lid-closed', label: 'Keep awake with the lid closed' },
];

/** The battery floors the operator can choose, in percent (AWK-006). */
export const FLOORS: readonly number[] = [10, 20, 30, 40, 50];

const PROBLEMS: Readonly<Record<StayAwakeProblem, string>> = {
  caller: 'Only the main window can change this.',
  unsaved: 'Anima.ing could not save this choice, so nothing changed.',
  unsupported: 'This computer has no set-up step.',
  refused: 'The administrator step was cancelled or did not finish. Nothing changed.',
  failed: 'The stay-awake control could not be changed.',
};

/** Fixed text for each problem; the shell's own message is never shown. */
export function problemText(problem: StayAwakeProblem): string {
  return PROBLEMS[problem];
}

const HELD: Readonly<Record<Level, string>> = {
  off: 'This computer sleeps as usual.',
  awake: 'This computer does not sleep while it is idle. The display can sleep.',
  'lid-closed': 'This computer stays on with the lid closed. A closed computer gets warm: keep it on a hard surface.',
};

/** What the computer does now, and why when that is less than the choice (AWK-006, AWK-008). */
export function describeStayAwake(status: StayAwakeStatus): string {
  switch (status.pause) {
    case 'low-battery':
      return `Off for now: the battery is below ${String(status.batteryFloor ?? 0)}%. It starts again on mains power.`;
    case 'lid-not-ready':
      return status.lid === 'needs-set-up'
        ? 'The lid-closed level is not set up. The computer is kept awake only while the lid is open.'
        : 'This computer refused the lid-closed level. It is kept awake only while the lid is open.';
    case 'unavailable':
      return 'This system refused to keep the computer awake, so it sleeps as usual.';
    case null:
      return HELD[status.held];
  }
}

const LOOPBACK_HOSTS: readonly string[] = ['localhost', '127.0.0.1', '[::1]'];

/**
 * Whether the hub this client talks to runs on this computer (AWK-005): its address is a loopback address, where the
 * in-app hub answers. The first-run flow of that hub must ask this function too. The rule cannot see a hub on this
 * computer that is reached by another address, and it takes a loopback port that forwards to another computer as this
 * one.
 */
export function hubIsThisComputer(origin: string): boolean {
  return URL.canParse(origin) && LOOPBACK_HOSTS.includes(new URL(origin).hostname);
}

/** The line that tells the operator that checks stop during sleep, or `null` when it does not apply (AWK-005). */
export function sleepNotice(thisComputerIsHub: boolean, status: StayAwakeStatus): string | null {
  return thisComputerIsHub && status.held === 'off' ? 'This computer is the hub. Its checks stop while the computer sleeps.' : null;
}

/** The floors the control offers: the fixed steps, and the saved floor when it is not one of them (AWK-006). */
export function floorChoices(saved: number | null): readonly number[] {
  return saved === null || FLOORS.includes(saved) ? FLOORS : [...FLOORS, saved].sort((a, b) => a - b);
}
