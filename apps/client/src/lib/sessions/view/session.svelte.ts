import type { HubError } from '$lib/hub/hub';
import type { SessionDisplay, SessionsApi } from '../api';
import { ending, recordedEnding, type SessionEnding, type TunnelClose } from '../ending';
import type { TunnelRefusal } from '../tunnel/state';
import type { TerminalLink } from '../tunnel/terminal/link';
import type { SessionPolicy } from '../transfer';
import type { ViewStartResult } from './start';
import type { Progress } from './steps/steps';

/** The K4 §5 close codes of a host key change and of an RDP certificate change. */
const HOST_KEY_CLOSE = 4001;
const CERTIFICATE_CLOSE = 4003;

/** What a connection reads from a tunnel's state: its phase and what it saw when it closed. */
export interface FollowedState extends TunnelClose {
  readonly phase: string;
}

/** A tunnel a connection follows, whose state is `State`. */
export interface FollowedTunnel<State extends FollowedState> {
  state(): State;
  close(): void;
}

/** Where one attempt stands: asking for a ticket, with an open tunnel, or ended. */
export type Stage<Tunnel = TerminalLink, State extends FollowedState = FollowedState> =
  | { readonly kind: 'starting' }
  | {
      readonly kind: 'open';
      readonly tunnel: Tunnel;
      readonly state: State;
      readonly connected: boolean;
      /** The copy and paste flags read with this attempt's ticket (RA-033). */
      readonly policy: SessionPolicy;
    }
  | { readonly kind: 'ending'; readonly ending: SessionEnding; readonly progress: Progress }
  | {
      readonly kind: 'ended';
      readonly ending: SessionEnding;
      readonly progress: Progress;
      /** A 4001 close or a recorded `HOST_KEY_MISMATCH` (D-6, SEC-007 and T1 note F-2). */
      readonly hostKeyChanged: boolean;
      /** A 4003 close or a recorded `SERVER_CERTIFICATE_MISMATCH`: the RDP certificate changed (K4 §3, §5). */
      readonly certificateChanged: boolean;
    };

/** What a connection hears from its tunnel; a view adds its tunnel's own hooks (size, output) to them. */
export interface ConnectionEvents<State> {
  readonly onchange: (state: State) => void;
  readonly onend: (state: State) => void;
}

/** Starts one attempt: asks for a ticket and opens the tunnel, reporting to `events` (see `SessionStart`). */
export type ConnectionStart<Tunnel, State extends FollowedState> = (
  display: SessionDisplay,
  events: ConnectionEvents<State>,
) => Promise<ViewStartResult<Tunnel>>;

/** Fixed text for a start the hub or the tunnel refused; never the hub's own message (LOG-007, R-035). */
const REFUSED: Readonly<Record<string, string>> = {
  CONFLICT: 'Could not open the session. The device has no accepted host key: run the connection test first.',
  NOT_FOUND: 'Could not open the session. The device no longer exists.',
  FORBIDDEN: 'Could not open the session. The hub does not allow it.',
  UNAUTHENTICATED: 'Could not open the session. Sign in to the hub again.',
  unreachable: 'Could not open the session. The hub did not answer.',
};
const REFUSED_OTHER = 'Could not open the session.';

function refusedEnding(error: HubError | TunnelRefusal, conflict: string | undefined): SessionEnding {
  if ('problem' in error) return { reason: undefined, opened: false, text: `${REFUSED_OTHER} ${error.message}` };
  const text = error.code === 'CONFLICT' && conflict !== undefined ? conflict : (REFUSED[error.code] ?? REFUSED_OTHER);
  return { reason: undefined, opened: false, text };
}

/** Tunnel states heard before the start answered. */
interface Early<State> {
  state?: State;
  ended?: State;
}

/** The steps' progress for `stage`. */
export function progressOf(stage: Stage<unknown, FollowedState>): Progress {
  if (stage.kind === 'starting') return { ticketed: false, opened: false, connected: false, stopped: false };
  if (stage.kind === 'open') return { ticketed: true, opened: stage.state.opened, connected: stage.connected, stopped: false };
  return stage.progress;
}

/**
 * One session view's connection (D-4, D-5): it starts through the `SessionStart` seam, follows the tunnel, and on
 * the end asks `endSession` for the recorded reason, which wins over the close code. Each `connect` is a new
 * attempt with a new ticket; events of an older attempt, or after `close`, are ignored. `conflict` is the text
 * for a `CONFLICT` refusal of the start; without it the host key text is shown.
 */
export class SessionConnection<Tunnel extends FollowedTunnel<State>, State extends FollowedState> {
  stage: Stage<Tunnel, State> = $state({ kind: 'starting' });
  #attempt = 0;
  #closed = false;
  readonly #start: ConnectionStart<Tunnel, State>;
  readonly #api: Pick<SessionsApi, 'endSession'>;
  readonly #onended: (ending: SessionEnding) => void;
  readonly #conflict: string | undefined;
  /** The text `stop` gave for the open attempt. */
  #stoppedWith: string | undefined;

  constructor(
    start: ConnectionStart<Tunnel, State>,
    api: Pick<SessionsApi, 'endSession'>,
    onended: (ending: SessionEnding) => void,
    conflict?: string,
  ) {
    this.#start = start;
    this.#api = api;
    this.#onended = onended;
    this.#conflict = conflict;
  }

  /** The open tunnel, while there is one. */
  get tunnel(): Tunnel | undefined {
    return this.stage.kind === 'open' ? this.stage.tunnel : undefined;
  }

  /** Whether the socket is open, so the terminal can be shown and typed into (D-7). */
  get ready(): boolean {
    return this.stage.kind === 'open' && this.stage.connected;
  }

  /**
   * Start the first attempt, or a new one once the last has ended ("Reconnect"), with `display`. Does nothing
   * after `close` or while an attempt is under way, so a double click asks for one ticket only.
   */
  async connect(display: SessionDisplay): Promise<void> {
    if (this.#closed || (this.#attempt > 0 && this.stage.kind !== 'ended')) return;
    const attempt = ++this.#attempt;
    this.#stoppedWith = undefined;
    this.stage = { kind: 'starting' };
    const early: Early<State> = {};
    const current = () => attempt === this.#attempt && !this.#closed;
    let sessionId = '';
    const result = await this.#start(display, {
      onchange: (state) => {
        early.state = state;
        if (current() && this.stage.kind === 'open') this.#follow(this.stage.tunnel, state);
      },
      onend: (state) => {
        early.ended = state;
        if (current() && this.stage.kind === 'open') void this.#end(attempt, sessionId, state);
      },
    });
    if (result.ok) sessionId = result.value.sessionId;
    this.#started(attempt, result, early);
  }

  /**
   * End the open attempt on the operator's side, as when a sign-in prompt is cancelled: close its tunnel, and when
   * it has ended (and `endSession` answered), show `text` instead of the reason the close would give.
   */
  stop(text: string): void {
    if (this.stage.kind !== 'open') return;
    this.#stoppedWith = text;
    this.stage.tunnel.close();
  }

  /** Close the open tunnel and ignore everything after; the hub records the end (K4 §5). */
  close(): void {
    this.#closed = true;
    this.tunnel?.close();
  }

  #started(attempt: number, result: ViewStartResult<Tunnel>, early: Early<State>): void {
    if (!result.ok) {
      if (attempt !== this.#attempt || this.#closed) return;
      const refused = refusedEnding(result.error, this.#conflict);
      // A tunnel refusal comes after the ticket was issued; a hub refusal is the ticket request itself.
      const ticketed = 'problem' in result.error;
      const progress = { ticketed, opened: false, connected: false, stopped: true };
      this.stage = { kind: 'ended', ending: refused, progress, hostKeyChanged: false, certificateChanged: false };
      this.#onended(refused);
      return;
    }
    const { tunnel, sessionId, policy } = result.value;
    // The view went away (or a newer attempt began) while the hub answered: this tunnel has no one to show it.
    if (attempt !== this.#attempt || this.#closed) {
      tunnel.close();
      return;
    }
    this.stage = { kind: 'open', tunnel, state: tunnel.state(), connected: false, policy };
    this.#follow(tunnel, early.state ?? tunnel.state());
    if (early.ended !== undefined) void this.#end(attempt, sessionId, early.ended);
  }

  #follow(tunnel: Tunnel, state: State): void {
    if (this.stage.kind !== 'open') return;
    const connected = this.stage.connected || state.phase === 'connected';
    this.stage = { kind: 'open', tunnel, state, connected, policy: this.stage.policy };
  }

  /** The text `stop` gave wins; otherwise the hub's recorded reason wins over the close (D-4). */
  #final(provisional: SessionEnding, endReason: SessionEnding['reason'] | null): SessionEnding {
    const stopped = this.#stoppedWith;
    if (stopped !== undefined) return { reason: 'OPERATOR', opened: provisional.opened, text: stopped };
    return recordedEnding(provisional, endReason);
  }

  async #end(attempt: number, sessionId: string, state: State): Promise<void> {
    const progress = { ...progressOf(this.stage), stopped: true };
    // A stopped attempt shows its own text at once, not the reason its close would give for a moment.
    const provisional = this.#final(ending(state), undefined);
    this.stage = { kind: 'ending', ending: provisional, progress };
    const recorded = await this.#api.endSession(sessionId);
    if (attempt !== this.#attempt || this.#closed) return;
    const final = this.#final(provisional, recorded.ok ? recorded.value.endReason : undefined);
    const hostKeyChanged = state.closeCode === HOST_KEY_CLOSE || final.reason === 'HOST_KEY_MISMATCH';
    const certificateChanged = state.closeCode === CERTIFICATE_CLOSE || final.reason === 'SERVER_CERTIFICATE_MISMATCH';
    this.stage = { kind: 'ended', ending: final, progress, hostKeyChanged, certificateChanged };
    this.#onended(final);
  }
}
