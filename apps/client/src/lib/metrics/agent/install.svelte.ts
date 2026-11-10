import type { HubError, HubResult } from '$lib/hub/hub';
import type { AgentEnrolment, AgentStatusView, metricsApi } from '$lib/metrics/api';
import { clearTime, secondsLeft } from './countdown';

/** The hub calls of the Install agent panel; the route passes `metricsApi` of its hub. */
export type AgentCalls = Pick<ReturnType<typeof metricsApi>, 'agentStatus' | 'enrolAgent' | 'revokeAgentToken'>;

/** Where the command is copied to; the browser's clipboard in the app. */
export interface ClipboardSink {
  writeText: (text: string) => Promise<void>;
}

/** What the panel reads from outside: the clock, the clipboard and a timer that returns the function that cancels it. */
export interface PanelEnv {
  readonly now: () => Date;
  readonly clipboard: ClipboardSink;
  readonly later: (run: () => void, milliseconds: number) => () => void;
}

/** How long "Copied." stays. */
const COPIED_MS = 4000;

/** The text last copied and whether the clipboard took it; `count` makes each copy a new announcement. */
export interface CopyOutcome {
  readonly text: string;
  readonly ok: boolean;
  readonly count: number;
}

/**
 * The Install agent panel's state for one device (MET-019, MET-018): the code's life (make, count down, clear) and the agent's
 * status (read, revoke). The enrolment code lives only in this object, which dies with the panel; it is cleared when it expires or
 * the operator closes it, and never reaches storage, the URL or a log (LOG-007). Time and timers come from `env`.
 */
export class AgentInstall {
  /** The code to show; `null` before one was made, after it expired and after `clear`. */
  code = $state<string | null>(null);
  /** Whole seconds until the code expires. */
  left = $state(0);
  expired = $state(false);
  busy = $state(false);
  error = $state<HubError | null>(null);
  copied = $state<CopyOutcome | null>(null);
  /** The device's agent state; `null` until read, or when it could not be. */
  status = $state<AgentStatusView | null>(null);
  statusError = $state<HubError | null>(null);
  /** True once the operator revoked the token in this panel: the source stays AGENT, stale, until it enrols again (R-092 D-7). */
  revoked = $state(false);

  private expiresAt: string | null = null;
  /** Counts the times the code was cleared or remade, so an answer for an earlier code is dropped. */
  private epoch = 0;
  private statusEpoch = 0;
  private copies = 0;
  private cancelCopied: () => void = () => undefined;

  constructor(
    private readonly calls: AgentCalls,
    private readonly deviceId: string,
    private readonly env: PanelEnv,
  ) {}

  /** Asks the hub for a fresh code (MET-012). */
  async makeCode(): Promise<void> {
    // A second code voids the first (K8 Section 3), so a request in flight is never doubled.
    if (this.busy) return;
    this.clear();
    const epoch = this.epoch;
    this.busy = true;
    const result = await this.calls.enrolAgent(this.deviceId);
    if (epoch !== this.epoch) return;
    this.busy = false;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.show(result.value);
  }

  private show({ code, expiresAt }: AgentEnrolment): void {
    this.expiresAt = clearTime(expiresAt, this.env.now());
    this.code = code;
    this.tick();
  }

  /** Recounts the time left; at zero the code is gone from the page (MET-012: 15 minutes). */
  tick(): void {
    if (this.expiresAt === null) return;
    this.left = secondsLeft(this.expiresAt, this.env.now());
    if (this.left > 0) return;
    this.clear();
    this.expired = true;
  }

  /** Forgets the code and anything about it; the panel calls it when it closes. */
  clear(): void {
    this.epoch += 1;
    this.code = null;
    this.expiresAt = null;
    this.left = 0;
    this.expired = false;
    this.busy = false;
    this.error = null;
    this.forgetCopy();
  }

  /** Copies `text`; the outcome shows for a few seconds and, as an announcement, again for each copy. */
  async copy(text: string): Promise<void> {
    let ok = true;
    try {
      await this.env.clipboard.writeText(text);
    } catch {
      ok = false;
    }
    this.copies += 1;
    this.copied = { text, ok, count: this.copies };
    this.cancelCopied();
    this.cancelCopied = this.env.later(() => {
      this.copied = null;
    }, COPIED_MS);
  }

  private forgetCopy(): void {
    this.cancelCopied();
    this.copied = null;
  }

  /** Reads the agent state; a failed read leaves no earlier state standing behind the error. */
  async loadStatus(): Promise<void> {
    this.statusEpoch += 1;
    const epoch = this.statusEpoch;
    const result = await this.calls.agentStatus(this.deviceId);
    if (epoch !== this.statusEpoch) return;
    this.statusError = result.ok ? null : result.error;
    this.status = result.ok ? result.value : null;
  }

  /** Revokes the token after the operator confirmed; the hub keeps the source AGENT, shown stale (R-092 D-7). */
  async revoke(): Promise<HubResult<AgentStatusView>> {
    const result = await this.calls.revokeAgentToken(this.deviceId);
    if (result.ok) {
      this.statusEpoch += 1;
      this.status = result.value;
      this.statusError = null;
      this.revoked = true;
    }
    return result;
  }
}
