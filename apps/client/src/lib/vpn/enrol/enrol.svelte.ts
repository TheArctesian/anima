import type { HubError } from '$lib/hub/hub';
import { secondsLeft } from '$lib/metrics/agent/countdown';
import type { EnrolApi, VpnKey, VpnRegistration } from './api';

/** What the modal shows: the choice, a key with its steps, the auth ID field, or the registered computer. */
export type EnrolView = 'choose' | 'key' | 'register' | 'registered';

/** Where the commands are copied to; the browser's clipboard in the app. */
export interface ClipboardSink {
  writeText: (text: string) => Promise<void>;
}

/** What the modal reads from outside: the clock, the clipboard and a timer that returns the function that cancels it. */
export interface EnrolEnv {
  readonly now: () => Date;
  readonly clipboard: ClipboardSink;
  readonly later: (run: () => void, milliseconds: number) => () => void;
}

/** How long "Copied." stays. */
const COPIED_MS = 4000;

/** The text last copied and whether the clipboard took it. */
export interface CopyOutcome {
  readonly text: string;
  readonly ok: boolean;
}

/**
 * The enrolment modal's state (VPN-001, VPN-003, VP-7): the pre-auth key's life (make, count down, clear) and the
 * registration by auth ID. The key lives only in this object, which dies with the modal. It is cleared when it expires,
 * when the step changes and when the modal closes, and never reaches storage, the URL or a log (LOG-007).
 */
export class Enrolment {
  view = $state<EnrolView>('choose');
  /** The key to show; `null` before one was made and after it was cleared. */
  key = $state<VpnKey | null>(null);
  /** Whole seconds until the key expires. */
  left = $state(0);
  expired = $state(false);
  busy = $state(false);
  error = $state<HubError | null>(null);
  registered = $state<VpnRegistration | null>(null);
  copied = $state<CopyOutcome | null>(null);

  private expiresAt: string | null = null;
  /** Counts each clear, so an answer for an earlier step is dropped. */
  private epoch = 0;
  private cancelCopied: () => void = () => undefined;

  constructor(
    private readonly api: EnrolApi,
    private readonly env: EnrolEnv,
  ) {}

  /** A key for the subnet router of `siteId` (VPN-001). */
  makeRouterKey(siteId: string): Promise<void> {
    return this.makeKey(() => this.api.createRouterKey(siteId));
  }

  /** A key for a Linux computer of the operator (VPN-003). */
  makeInvite(): Promise<void> {
    return this.makeKey(() => this.api.createInvite());
  }

  private async makeKey(call: () => ReturnType<EnrolApi['createInvite']>): Promise<void> {
    if (this.busy) return;
    this.clear();
    const epoch = this.epoch;
    this.busy = true;
    const result = await call();
    if (epoch !== this.epoch) return;
    this.busy = false;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.expiresAt = result.value.expiresAt;
    this.key = result.value;
    this.view = 'key';
    this.tick();
  }

  /** Recounts the time left; at zero the key is gone from the page. */
  tick(): void {
    if (this.expiresAt === null) return;
    this.left = secondsLeft(this.expiresAt, this.env.now());
    if (this.left > 0) return;
    this.clear();
    this.expired = true;
    this.view = 'choose';
  }

  /** Moves to the auth ID field, for a Mac or another computer (vpn.md 6B). */
  openRegister(): void {
    this.clear();
    this.view = 'register';
  }

  /** Registers the computer that shows `authId` on its own screen. */
  async register(authId: string): Promise<void> {
    if (this.busy) return;
    const epoch = this.epoch;
    this.busy = true;
    this.error = null;
    const result = await this.api.register(authId.trim());
    if (epoch !== this.epoch) return;
    this.busy = false;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.registered = result.value;
    this.view = 'registered';
  }

  /** Back to the choice; the key goes with the step. */
  choose(): void {
    this.clear();
    this.view = 'choose';
  }

  /** The key was used (a router showed up): it is one-use, so it leaves the page. */
  keyUsed(): void {
    this.key = null;
    this.expiresAt = null;
    this.left = 0;
    this.forgetCopy();
  }

  /** Forgets the key and everything about it; the modal calls it when it closes. */
  close(): void {
    this.clear();
    this.registered = null;
    this.view = 'choose';
  }

  private clear(): void {
    this.epoch += 1;
    this.keyUsed();
    this.expired = false;
    this.busy = false;
    this.error = null;
  }

  /** Copies `text`; the outcome shows for a few seconds. */
  async copy(text: string): Promise<void> {
    let ok = true;
    try {
      await this.env.clipboard.writeText(text);
    } catch {
      ok = false;
    }
    this.copied = { text, ok };
    this.cancelCopied();
    this.cancelCopied = this.env.later(() => {
      this.copied = null;
    }, COPIED_MS);
  }

  private forgetCopy(): void {
    this.cancelCopied();
    this.copied = null;
  }
}
