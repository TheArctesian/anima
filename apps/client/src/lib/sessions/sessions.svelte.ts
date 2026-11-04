import { createContext } from 'svelte';
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

/** The display the operator's pane offers; K1 `DisplayInput` (D-7, GW-012). */
export interface SessionDisplay {
  readonly widthPx: number;
  readonly heightPx: number;
  readonly dpi?: number;
  readonly timezone?: string;
}

/**
 * What a started session gives back. The hub's ticket also carries the single-use token
 * (API-009); the store passes it to the caller and never keeps it (D-3, LOG-007).
 */
export interface StartedSession {
  readonly sessionId: string;
}

/** The hub calls the store makes; the root layout supplies them (K1 `startSession`, `endSession`). */
export interface SessionPort {
  start(deviceId: string, display: SessionDisplay): Promise<StartedSession>;
  end(sessionId: string): Promise<unknown>;
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

/** A start finished after its tab was closed or started again; its session has been ended. */
export class TabClosedError extends Error {
  constructor(cause?: unknown) {
    super('The session tab was closed before the session started', cause === undefined ? undefined : { cause });
    this.name = 'TabClosedError';
  }
}

/** The open session tabs (RA-001): order, selection and the hub sessions behind them. */
export interface Sessions {
  readonly tabs: readonly Tab[];
  readonly active: Tab | null;
  /** Whether "Pop out" and drag-out may be offered (D-13). */
  readonly canPopOut: boolean;
  openSession(request: SessionRequest): TabId;
  /** Starts a hub session in tab `id` and returns its ticket; a restart ends the previous session first (D-5). */
  start(id: TabId, display: SessionDisplay): Promise<StartedSession>;
  select(id: TabId): void;
  deselect(): void;
  /** Removes the tab and ends its session once; rejects when the hub could not end it. */
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

  async start(id: TabId, display: SessionDisplay): Promise<StartedSession> {
    const tab = this.#find(id);
    if (tab === undefined) throw new TabClosedError();
    this.#starts += 1;
    const mine = this.#starts;
    this.#latestStart.set(id, mine);
    if (tab.sessionId !== null) {
      await this.#endPrevious(id, tab.sessionId);
      if (!this.#isLatest(id, mine)) throw new TabClosedError();
    }
    const started = await this.#port.start(tab.deviceId, display);
    if (!this.#isLatest(id, mine)) {
      return this.#endAfterClose(started.sessionId);
    }
    this.#state = withSession(this.#state, id, started.sessionId);
    return started;
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
    // A start still in flight ends its own session when it resolves (see `start`).
    if (tab.sessionId !== null) await this.#port.end(tab.sessionId);
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
      if (this.#poppingOut.get(id) === true) await this.#port.end(sessionId);
      throw reason;
    } finally {
      this.#poppingOut.delete(id);
    }
    this.#remove(id);
  }

  #isLatest(id: TabId, start: number): boolean {
    return this.#find(id) !== undefined && this.#latestStart.get(id) === start;
  }

  /**
   * A restart ends the tab's previous session once before the new one starts (D-5). The tab
   * gives the id up first, so a close or another start meanwhile cannot end it again.
   */
  async #endPrevious(id: TabId, previous: string): Promise<void> {
    this.#state = withSession(this.#state, id, null);
    try {
      await this.#port.end(previous);
    } catch (reason) {
      if (this.#find(id)?.sessionId === null) this.#state = withSession(this.#state, id, previous);
      throw reason;
    }
  }

  #find(id: TabId | null): Tab | undefined {
    return this.#state.tabs.find((tab) => tab.id === id);
  }

  #remove(id: TabId): void {
    this.#state = closed(this.#state, id);
    this.#latestStart.delete(id);
  }

  async #endAfterClose(sessionId: string): Promise<never> {
    const failure = await this.#port.end(sessionId).then(
      () => undefined,
      (reason: unknown) => reason,
    );
    throw new TabClosedError(failure);
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
