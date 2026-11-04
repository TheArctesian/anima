import { createContext } from 'svelte';
import type { HubError, HubResult } from '$lib/hub/hub';
import type { TunnelStart } from './tunnel/start';
import type { SessionStart, TunnelEvents } from './view/start';
import {
  closed,
  deselected,
  moved,
  noTabs,
  opened,
  selected,
  withSession,
  type Origin,
  type SessionProtocol,
  type Tab,
  type TabId,
  type Tabs,
} from './tabs';

/** The hub calls behind the store; the root layout supplies them (K1 `startSession`, `endSession`). */
export interface SessionPort {
  /**
   * How a session view starts sessions on `deviceId` (D-5, D-7):
   * `sessionStart(sessionsApi(hub), deviceId, hub.address.origin)`. Only the view calls it, because only the
   * view knows its display; the store wraps it to learn each session ID (see `startIn`).
   */
  start(deviceId: string): SessionStart;
  /** End the session, or return an ended one unchanged (K1 `endSession`, D-4). */
  end(sessionId: string): Promise<HubResult<unknown>>;
}

/** Moves a live session into its own window (RA-002, D-13); only the desktop shell has one (WP-16). */
export type PopOutPort = (sessionId: string) => Promise<void>;

/** What `openSession` needs: the device, the protocol and the control that opened it (H-1). */
export interface SessionRequest {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly protocol: SessionProtocol;
  readonly origin: Origin | null;
}

/** The hub could not end a session; `code` is its K7 code. The text is fixed (LOG-007, R-035). */
export class SessionEndError extends Error {
  readonly code: HubError['code'];

  constructor(error: HubError) {
    super('The hub could not end the session');
    this.name = 'SessionEndError';
    this.code = error.code;
  }
}

/** What a start in a tab that has gone answers; the view of that tab has gone too, so no one shows it. */
const TAB_CLOSED: TunnelStart = { ok: false, error: { problem: 'not-opened', message: 'The session tab was closed.' } };

/** What one start in a tab has learnt from its tunnel so far. */
interface Attempt {
  sessionId: string | undefined;
  /** The tunnel ended, so the view ends the session itself to learn the recorded reason (D-4). */
  ended: boolean;
}

/** The open session tabs (RA-001): order, selection and the hub sessions behind them. */
export interface Sessions {
  readonly tabs: readonly Tab[];
  readonly active: Tab | null;
  /** Whether "Pop out" and drag-out may be offered (D-13). */
  readonly canPopOut: boolean;
  /**
   * Opens and selects a tab for `request`; its view then starts the session (D-11). The one entry point: the device
   * page's SSH button calls `useSessions().openSession({ deviceId, deviceName, protocol: 'SSH', origin:
   * button.getBoundingClientRect() })` (WP-14 H-1, WP-12).
   */
  openSession(request: SessionRequest): TabId;
  /**
   * The `SessionStart` for the view in tab `id`. Each call starts a hub session and records its ID on the tab;
   * one that replaces a live session ends that first (D-5); one that finishes after the tab closed, or after a
   * newer start, has its session ended here. A session whose tunnel ended is the view's to end (D-4).
   */
  startIn(id: TabId): SessionStart;
  select(id: TabId): void;
  deselect(): void;
  /** Removes the tab and ends its session once; rejects with `SessionEndError` when the hub could not end it. */
  close(id: TabId): Promise<void>;
  move(from: number, to: number): void;
  /** Hands the tab's session to a new window and removes the tab without ending the session. */
  popOut(id: TabId): Promise<void>;
}

class OpenSessions implements Sessions {
  #state = $state.raw<Tabs>(noTabs);
  #opens = 0;
  #starts = 0;
  readonly #latestStart = new Map<TabId, number>();
  /** Tabs whose session is being handed to a window; `true` once the operator closed the tab meanwhile. */
  readonly #poppingOut = new Map<TabId, boolean>();
  readonly #port: SessionPort;
  readonly #popOut: PopOutPort | undefined;
  readonly canPopOut: boolean;

  constructor(port: SessionPort, popOut: PopOutPort | undefined) {
    this.#port = port;
    this.#popOut = popOut;
    this.canPopOut = popOut !== undefined;
  }

  get tabs(): readonly Tab[] {
    return this.#state.tabs;
  }

  get active(): Tab | null {
    return this.#find(this.#state.activeId) ?? null;
  }

  openSession({ deviceId, deviceName, protocol, origin }: SessionRequest): TabId {
    this.#opens += 1;
    const id = `tab-${String(this.#opens)}`;
    this.#state = opened(this.#state, { id, deviceId, deviceName, protocol, origin, sessionId: null });
    return id;
  }

  startIn(id: TabId): SessionStart {
    return async (display, events) => {
      const tab = this.#find(id);
      if (tab === undefined) return TAB_CLOSED;
      this.#starts += 1;
      const mine = this.#starts;
      this.#latestStart.set(id, mine);
      if (tab.sessionId !== null) {
        const failure = await this.#endPrevious(id, tab.sessionId);
        if (failure !== undefined) return { ok: false, error: failure };
        if (!this.#isLatest(id, mine)) return TAB_CLOSED;
      }
      const attempt: Attempt = { sessionId: undefined, ended: false };
      const result = await this.#port.start(tab.deviceId)(display, this.#following(id, attempt, events));
      if (!result.ok) return result;
      attempt.sessionId = result.value.sessionId;
      if (!this.#isLatest(id, mine)) {
        // The view that asked has gone or moved on, so nothing owns this session: close its tunnel now (the view
        // may close it too; that is safe) and end it. A failure has no one to tell: the hub also records the end
        // when the tunnel closes (K4 §5).
        result.value.tunnel.close();
        await this.#port.end(attempt.sessionId);
      } else if (!attempt.ended) {
        this.#state = withSession(this.#state, id, attempt.sessionId);
      }
      return result;
    };
  }

  select(id: TabId): void {
    this.#state = selected(this.#state, id);
  }

  deselect(): void {
    this.#state = deselected(this.#state);
  }

  async close(id: TabId): Promise<void> {
    const tab = this.#find(id);
    if (tab === undefined) return;
    this.#remove(id);
    // A session on its way to a window is the window's unless the hand-over fails (see `popOut`).
    if (this.#poppingOut.has(id)) {
      this.#poppingOut.set(id, true);
      return;
    }
    // A start still in flight ends its own session when it resolves (see `startIn`).
    if (tab.sessionId !== null) await this.#end(tab.sessionId);
  }

  move(from: number, to: number): void {
    this.#state = moved(this.#state, from, to);
  }

  async popOut(id: TabId): Promise<void> {
    const sessionId = this.#find(id)?.sessionId;
    if (this.#popOut === undefined || sessionId == null) return;
    this.#poppingOut.set(id, false);
    try {
      await this.#popOut(sessionId);
    } catch (reason) {
      // The tab was closed while the window was opening: nothing owns the session now.
      if (this.#poppingOut.get(id) === true) await this.#end(sessionId);
      throw reason;
    } finally {
      this.#poppingOut.delete(id);
    }
    this.#remove(id);
  }

  #isLatest(id: TabId, start: number): boolean {
    return this.#find(id) !== undefined && this.#latestStart.get(id) === start;
  }

  /** `events`, after the tab gives up the session of `attempt` when its tunnel ends (D-4). */
  #following(id: TabId, attempt: Attempt, events: TunnelEvents): TunnelEvents {
    return {
      onchange: events.onchange,
      onend: (state) => {
        attempt.ended = true;
        if (attempt.sessionId !== undefined && this.#find(id)?.sessionId === attempt.sessionId) {
          this.#state = withSession(this.#state, id, null);
        }
        events.onend(state);
      },
    };
  }

  /**
   * A new start ends the tab's live session once before it asks for another (D-5). The tab gives the id up first,
   * so a close or another start meanwhile cannot end it again; on a failure it takes the id back.
   */
  async #endPrevious(id: TabId, previous: string): Promise<HubError | undefined> {
    this.#state = withSession(this.#state, id, null);
    const ended = await this.#port.end(previous);
    if (ended.ok) return undefined;
    if (this.#find(id)?.sessionId === null) this.#state = withSession(this.#state, id, previous);
    return ended.error;
  }

  async #end(sessionId: string): Promise<void> {
    const ended = await this.#port.end(sessionId);
    if (!ended.ok) throw new SessionEndError(ended.error);
  }

  #find(id: TabId | null): Tab | undefined {
    return this.#state.tabs.find((tab) => tab.id === id);
  }

  #remove(id: TabId): void {
    this.#state = closed(this.#state, id);
    this.#latestStart.delete(id);
  }
}

/** The session store over `port`; pass `popOut` only where a window can take a session (D-13). */
export function createSessions(port: SessionPort, popOut?: PopOutPort): Sessions {
  return new OpenSessions(port, popOut);
}

const [getSessions, setSessions] = createContext<Sessions>();

/** Make `sessions` available to every component below; call once in the root layout. */
export function provideSessions(sessions: Sessions): void {
  setSessions(sessions);
}

/** The sessions set by `provideSessions` in a parent component; throws when there is none. */
export function useSessions(): Sessions {
  return getSessions();
}
