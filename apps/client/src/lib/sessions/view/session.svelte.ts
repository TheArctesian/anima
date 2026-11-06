import type { HubError } from '$lib/hub/hub';
import type { SessionDisplay, SessionsApi } from '../api';
import { ending, recordedEnding, type SessionEnding } from '../ending';
import type { TunnelStart } from '../tunnel/start';
import type { TunnelRefusal, TunnelState } from '../tunnel/state';
import type { TerminalLink } from '../tunnel/terminal/link';
import type { TunnelEvents } from './start';
import type { Progress } from './steps/steps';

/** The K4 §8.6 close code of a host key change. */
const HOST_KEY_CLOSE = 4001;

/** Where one attempt stands: asking for a ticket, with an open tunnel, or ended. */
export type Stage =
  | { readonly kind: 'starting' }
  | { readonly kind: 'open'; readonly tunnel: TerminalLink; readonly state: TunnelState; readonly connected: boolean }
  | { readonly kind: 'ending'; readonly ending: SessionEnding; readonly progress: Progress }
  | {
      readonly kind: 'ended';
      readonly ending: SessionEnding;
      readonly progress: Progress;
      /** A 4001 close or a recorded `HOST_KEY_MISMATCH` (D-6, SEC-007 and T1 note F-2). */
      readonly hostKeyChanged: boolean;
    };

/** What a connection hears from its tunnel; the view adds the terminal's own hooks (size, output) to them. */
export type ConnectionEvents = Pick<TunnelEvents, 'onchange' | 'onend'>;

/** Starts one attempt: asks for a ticket and opens the tunnel, reporting to `events` (see `SessionStart`). */
export type ConnectionStart = (display: SessionDisplay, events: ConnectionEvents) => Promise<TunnelStart>;

/** Fixed text for a start the hub or the tunnel refused; never the hub's own message (LOG-007, R-035). */
const REFUSED: Readonly<Record<string, string>> = {
  CONFLICT: 'Could not open the session. The device has no accepted host key: run the connection test first.',
  NOT_FOUND: 'Could not open the session. The device no longer exists.',
  FORBIDDEN: 'Could not open the session. The hub does not allow it.',
  UNAUTHENTICATED: 'Could not open the session. Sign in to the hub again.',
  unreachable: 'Could not open the session. The hub did not answer.',
};
const REFUSED_OTHER = 'Could not open the session.';

function refusedEnding(error: HubError | TunnelRefusal): SessionEnding {
  const text = 'problem' in error ? `${REFUSED_OTHER} ${error.message}` : (REFUSED[error.code] ?? REFUSED_OTHER);
  return { reason: undefined, opened: false, text };
}

/** The steps' progress for `stage`. */
export function progressOf(stage: Stage): Progress {
  if (stage.kind === 'starting') return { ticketed: false, opened: false, connected: false, stopped: false };
  if (stage.kind === 'open') return { ticketed: true, opened: stage.state.opened, connected: stage.connected, stopped: false };
  return stage.progress;
}

/**
 * One session view's connection (D-4, D-5): it starts through the `SessionStart` seam, follows the tunnel, and on
 * the end asks `endSession` for the recorded reason, which wins over the close code. Each `connect` is a new
 * attempt with a new ticket; events of an older attempt, or after `close`, are ignored.
 */
export class SessionConnection {
  stage: Stage = $state({ kind: 'starting' });
  #attempt = 0;
  #closed = false;
  readonly #start: ConnectionStart;
  readonly #api: Pick<SessionsApi, 'endSession'>;
  readonly #onended: (ending: SessionEnding) => void;

  constructor(start: ConnectionStart, api: Pick<SessionsApi, 'endSession'>, onended: (ending: SessionEnding) => void) {
    this.#start = start;
    this.#api = api;
    this.#onended = onended;
  }

  /** The open tunnel, while there is one. */
  get tunnel(): TerminalLink | undefined {
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
    this.stage = { kind: 'starting' };
    const early: { state?: TunnelState; ended?: TunnelState } = {};
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

  /** Close the open tunnel and ignore everything after; the hub records the end (K4 §5). */
  close(): void {
    this.#closed = true;
    this.tunnel?.close();
  }

  #started(attempt: number, result: TunnelStart, early: { state?: TunnelState; ended?: TunnelState }): void {
    if (!result.ok) {
      if (attempt !== this.#attempt || this.#closed) return;
      const refused = refusedEnding(result.error);
      // A tunnel refusal comes after the ticket was issued; a hub refusal is the ticket request itself.
      const ticketed = 'problem' in result.error;
      const progress = { ticketed, opened: false, connected: false, stopped: true };
      this.stage = { kind: 'ended', ending: refused, progress, hostKeyChanged: false };
      this.#onended(refused);
      return;
    }
    const { tunnel, sessionId } = result.value;
    // The view went away (or a newer attempt began) while the hub answered: this tunnel has no one to show it.
    if (attempt !== this.#attempt || this.#closed) {
      tunnel.close();
      return;
    }
    this.stage = { kind: 'open', tunnel, state: tunnel.state(), connected: false };
    this.#follow(tunnel, early.state ?? tunnel.state());
    if (early.ended !== undefined) void this.#end(attempt, sessionId, early.ended);
  }

  #follow(tunnel: TerminalLink, state: TunnelState): void {
    const connected = (this.stage.kind === 'open' && this.stage.connected) || state.phase === 'connected';
    this.stage = { kind: 'open', tunnel, state, connected };
  }

  async #end(attempt: number, sessionId: string, state: TunnelState): Promise<void> {
    const progress = { ...progressOf(this.stage), stopped: true };
    const provisional = ending(state);
    this.stage = { kind: 'ending', ending: provisional, progress };
    const recorded = await this.#api.endSession(sessionId);
    if (attempt !== this.#attempt || this.#closed) return;
    const final = recorded.ok ? recordedEnding(provisional, recorded.value.endReason) : provisional;
    const hostKeyChanged = state.closeCode === HOST_KEY_CLOSE || final.reason === 'HOST_KEY_MISMATCH';
    this.stage = { kind: 'ended', ending: final, progress, hostKeyChanged };
    this.#onended(final);
  }
}
