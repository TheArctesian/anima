import type { ThisComputerShell } from '$lib/desktop/agent/shell';
import { isThisComputer } from '$lib/devices/screen-sharing/setup';
import type { AgentCalls } from './install.svelte';

/** The device the panel is for: its ID for the hub and its host, which tells whether it is this computer (R-060). */
export interface PanelDevice {
  readonly id: string;
  readonly host: string;
}

/**
 * "Install on this computer" in the desktop app (MET-019, D-10 d): offered when the device is the computer the app runs on
 * and the app carries the agent. A fresh code goes from the hub straight to the shell. It is never kept in state, shown or
 * copied, and nothing refers to it once the shell answers (LOG-007).
 */
export class ThisComputerInstall {
  /** True once the device is known to be this computer and the app carries the agent. */
  offered = $state(false);
  busy = $state(false);
  /** Why the last install failed; `null` before one and after a success. */
  error = $state<string | null>(null);
  installed = $state(false);

  /** Counts disposals, so an answer that arrives after the panel closed changes nothing. */
  private epoch = 0;

  constructor(
    private readonly calls: Pick<AgentCalls, 'enrolAgent'>,
    private readonly shell: ThisComputerShell | null,
    private readonly device: PanelDevice,
    private readonly origin: string,
  ) {}

  /** Finds out whether to offer the install; outside the desktop app it never is. */
  async check(): Promise<void> {
    if (this.shell === null) return;
    const epoch = this.epoch;
    const machine = await this.shell.thisMachine();
    if (epoch !== this.epoch || !isThisComputer(this.device, machine)) return;
    const bundled = await this.shell.bundled();
    if (epoch === this.epoch) this.offered = bundled;
  }

  /** Makes a code and has the shell install and enrol the agent with it. Returns whether it is installed. */
  async install(): Promise<boolean> {
    if (this.shell === null || !this.offered || this.busy) return false;
    const epoch = this.epoch;
    this.busy = true;
    this.error = null;
    this.installed = false;
    const error = await this.installWithFreshCode(this.shell, epoch);
    if (epoch !== this.epoch) return false;
    this.busy = false;
    this.error = error;
    this.installed = error === null;
    return this.installed;
  }

  /**
   * The only place the code is held: a local of this call, gone when it returns. `null` on success, else the message. A code
   * that arrives after the panel closed never goes to the shell.
   */
  private async installWithFreshCode(shell: ThisComputerShell, epoch: number): Promise<string | null> {
    const made = await this.calls.enrolAgent(this.device.id);
    if (epoch !== this.epoch) return null;
    if (!made.ok) return made.error.message;
    const done = await shell.install(made.value.code, this.origin);
    return done.ok ? null : done.message;
  }

  /** The panel closes: later answers are dropped. */
  dispose(): void {
    this.epoch += 1;
  }
}
