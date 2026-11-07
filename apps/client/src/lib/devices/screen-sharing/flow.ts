import type { DeviceDetail } from '$lib/devices/api';
import { SystemSettingsError } from '$lib/desktop/systemSettings';
import {
  enableVnc,
  isThisComputer,
  NO_ANSWER_TEXT,
  screenSharingTest,
  type ScreenSharingCalls,
  type SetupDevice,
  type SetupState,
} from './setup';

type Answer = { readonly kind: 'reachable' } | { readonly kind: 'unreachable' } | { readonly kind: 'refused'; readonly message: string };

const SETTINGS_NOT_OPENED = 'System Settings did not open.';

/**
 * The one-button flow of R-079 for one device at a time: test port 5900, turn VNC on and hand the device over; for
 * this computer, open Sharing and test again every interval until it answers. `cancel` stops everything, and an
 * answer from an earlier run is dropped.
 */
export class SetupFlow {
  readonly #calls: ScreenSharingCalls;
  readonly #notify: (state: SetupState) => void;
  readonly #ready: (device: DeviceDetail) => void;
  #run = 0;
  #kind: SetupState['kind'] = 'idle';
  #stopTimer: (() => void) | undefined;

  constructor(calls: ScreenSharingCalls, show: (state: SetupState) => void, ready: (device: DeviceDetail) => void) {
    this.#calls = calls;
    this.#notify = show;
    this.#ready = ready;
  }

  async start(device: SetupDevice): Promise<void> {
    const run = this.#begin();
    this.#show({ kind: 'testing' });
    const machine = await this.#calls.thisMachine();
    if (run !== this.#run) return;
    const here = isThisComputer(device, machine);
    const user = here ? machine.user : null;
    const answer = await this.#probe(device);
    if (run !== this.#run) return;
    if (answer.kind === 'unreachable' && here && this.#calls.sharingAvailable()) {
      this.#show({ kind: 'waiting', problem: null });
      this.#schedule(run, device, user);
      await this.#open(run);
      return;
    }
    await this.#settle(run, device, user, answer);
  }

  /** Opens Sharing again while the flow waits, after a failure to open it. */
  reopenSharing(): Promise<void> {
    return this.#open(this.#run);
  }

  async #open(run: number): Promise<void> {
    let problem: string | null = null;
    try {
      await this.#calls.openSharing();
    } catch (reason) {
      problem = reason instanceof SystemSettingsError ? reason.message : SETTINGS_NOT_OPENED;
    }
    // A retest may have moved the flow on while System Settings was opening; the late answer must not undo that.
    if (run === this.#run && this.#kind === 'waiting') this.#show({ kind: 'waiting', problem });
  }

  /** Stops the repeats and drops any answer still on its way. */
  cancel(): void {
    this.#begin();
    this.#show({ kind: 'idle' });
  }

  /** Stops without showing anything: the owner is going away. */
  dispose(): void {
    this.#begin();
  }

  #show(state: SetupState): void {
    this.#kind = state.kind;
    this.#notify(state);
  }

  #begin(): number {
    this.#stopTimer?.();
    this.#stopTimer = undefined;
    this.#run += 1;
    return this.#run;
  }

  async #probe(device: SetupDevice): Promise<Answer> {
    const result = await this.#calls.test(screenSharingTest(device));
    if (!result.ok) return { kind: 'refused', message: result.error.message };
    return { kind: result.value.reachable ? 'reachable' : 'unreachable' };
  }

  #schedule(run: number, device: SetupDevice, user: string | null): void {
    this.#stopTimer = this.#calls.after(this.#calls.intervalMs, () => {
      void this.#retest(run, device, user);
    });
  }

  async #retest(run: number, device: SetupDevice, user: string | null): Promise<void> {
    if (run !== this.#run) return;
    const answer = await this.#probe(device);
    if (run !== this.#run) return;
    if (answer.kind === 'unreachable') this.#schedule(run, device, user);
    else await this.#settle(run, device, user, answer);
  }

  async #settle(run: number, device: SetupDevice, user: string | null, answer: Answer): Promise<void> {
    if (answer.kind !== 'reachable') {
      this.#show({ kind: 'failed', message: answer.kind === 'refused' ? answer.message : NO_ANSWER_TEXT });
      return;
    }
    this.#show({ kind: 'starting' });
    const result = await this.#calls.update(device.id, enableVnc(user));
    if (run !== this.#run) return;
    if (!result.ok) {
      this.#show({ kind: 'failed', message: result.error.message });
      return;
    }
    this.#show({ kind: 'idle' });
    this.#ready(result.value);
  }
}
