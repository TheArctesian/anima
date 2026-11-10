import { createContext } from 'svelte';
import type { OperatingSystem, Status } from '$lib/hub/generated/graphql';
import type { HubError, HubResult } from '$lib/hub/hub';
import type { TunnelRefusal } from './tunnel/state';
import type { DeviceStarts, RemoteStart, SessionStart, ViewStartResult } from './view/start';
import {
  closed,
  deselected,
  moved,
  noTabs,
  opened,
  selected,
  titled,
  withSession,
  type Origin,
  type SessionProtocol,
  type Tab,
  type WebTarget,
  type TabId,
  type Tabs,
} from './tabs';

/** The hub calls behind the store; the root layout supplies them (K1 `startSession`, `endSession`). */
export interface SessionPort {
  /**
   * How a session view starts sessions on `deviceId` (D-5, D-7), one start per tunnel:
   * `sessionStart(sessionsApi(hub), deviceId, hub.address.origin)`. Only the view calls it, because only the
   * view knows its display; the store wraps it to learn each session ID (see `startIn`).
   */
  start(deviceId: string): DeviceStarts;
  /** End the session, or return an ended one unchanged (K1 `endSession`, D-4). */
  end(sessionId: string): Promise<HubResult<unknown>>;
}

/**
 * Opens a window that starts its own session on `device` through `openSession` (RA-002, D-13); only the desktop
 * shell has one (WP-16). A hub session cannot change hands (K4: one ticket per `startSession`), so the port is never
 * given a session ID and the tab's session ends (`OPERATOR`).
 */
export type PopOutPort = (device: PoppedOutDevice) => Promise<void>;

/** What a new window needs to open its own session with `openSession` (D-13): the device, never a session ID. */
export interface PoppedOutDevice {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly protocol: SessionProtocol;
}

/** What `openSession` needs: the device, the protocol and the control that opened it (H-1). */
export interface SessionRequest {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly os?: OperatingSystem;
  readonly distribution?: string | null;
  readonly protocol: SessionProtocol;
  readonly origin: Origin | null;
}

/** What `openWeb` needs: the interface, its device and the control that opened it (WEB-003, MOT-005). */
export interface WebRequest {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly os?: OperatingSystem;
  readonly distribution?: string | null;
  readonly status: Status;
  readonly web: WebTarget;
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

interface Refused {
  readonly ok: false;
  readonly error: TunnelRefusal;
}

/** What a start in a tab that has gone answers; the view of that tab has gone too, so no one shows it. */
const TAB_CLOSED: Refused = { ok: false, error: { problem: 'not-opened', message: 'The session tab was closed.' } };

/** What a start through the other tunnel's view answers: an SSH tab has no remote display, and the reverse. */
const OTHER_VIEW: Refused = { ok: false, error: { problem: 'not-opened', message: 'This session tab uses another protocol.' } };

/** Starts one attempt; `ended` is called when its tunnel ends, before the view hears it. */
type Launch<Tunnel> = (ended: () => void) => Promise<ViewStartResult<Tunnel>>;

/** `onend`, after `ended`. */
function afterEnded<State>(ended: () => void, onend: (state: State) => void): (state: State) => void {
  return (state) => {
    ended();
    onend(state);
  };
}

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
  /** Opens and selects a web tab for `request` (WEB-003); its pane starts the page and reads the pin (WEB-008). */
  openWeb(request: WebRequest): TabId;
  /**
   * The `SessionStart` for the SSH view in tab `id`. Each call starts a hub session and records its ID on the tab;
   * one that replaces a live session ends that first (D-5); one that finishes after the tab closed, or after a
   * newer start, has its session ended here. A session whose tunnel ended is the view's to end (D-4). In an RDP
   * or VNC tab it starts nothing.
   */
  startIn(id: TabId): SessionStart;
  /** The same for the remote view of an RDP or VNC tab, which asks `startSession` for the tab's protocol. */
  remoteStartIn(id: TabId): RemoteStart;
  select(id: TabId): void;
  deselect(): void;
  /** Show `title` (the remote shell's, cleaned) in the tab; `''` clears it. */
  retitle(id: TabId, title: string): void;
  /** Removes the tab and ends its session once; rejects with `SessionEndError` when the hub could not end it. */
  close(id: TabId): Promise<void>;
  move(from: number, to: number): void;
  /**
   * Opens the tab's device in a new window, then removes the tab and ends its session once; the window starts its
   * own. Rejects with the port's error and keeps the tab when no window opens, or with `SessionEndError` (the tab
   * gone) when the hub could not end the session.
   */
  popOut(id: TabId): Promise<void>;
}

class OpenSessions implements Sessions {
  #state = $state.raw<Tabs>(noTabs);
  #opens = 0;
  #starts = 0;
  readonly #latestStart = new Map<TabId, number>();
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

  openSession({ deviceId, deviceName, os, distribution, protocol, origin }: SessionRequest): TabId {
    this.#opens += 1;
    const id = `tab-${String(this.#opens)}`;
    this.#state = opened(this.#state, { id, deviceId, deviceName, ...(os === undefined ? {} : { os }), ...(distribution == null ? {} : { distribution }), protocol, origin, sessionId: null });
    return id;
  }

  openWeb({ deviceId, deviceName, os, distribution, status, web, origin }: WebRequest): TabId {
    this.#opens += 1;
    const id = `tab-${String(this.#opens)}`;
    this.#state = opened(this.#state, { id, deviceId, deviceName, ...(os === undefined ? {} : { os }), ...(distribution == null ? {} : { distribution }), protocol: 'WEB', origin, sessionId: null, web, status });
    return id;
  }

  startIn(id: TabId): SessionStart {
    return (display, events) =>
      this.#begin(id, (tab) => {
        if (tab.protocol !== 'SSH') return undefined;
        const start = this.#port.start(tab.deviceId).terminal;
        return (ended) => start(display, { ...events, onend: afterEnded(ended, events.onend) });
      });
  }

  remoteStartIn(id: TabId): RemoteStart {
    return (display, events) =>
      this.#begin(id, (tab) => {
        if (tab.protocol === 'SSH' || tab.protocol === 'WEB') return undefined;
        const start = this.#port.start(tab.deviceId).remote(tab.protocol);
        return (ended) => start(display, { ...events, onend: afterEnded(ended, events.onend) });
      });
  }

  select(id: TabId): void {
    this.#state = selected(this.#state, id);
  }

  retitle(id: TabId, title: string): void {
    this.#state = titled(this.#state, id, title);
  }

  deselect(): void {
    this.#state = deselected(this.#state);
  }

  async close(id: TabId): Promise<void> {
    const tab = this.#find(id);
    if (tab === undefined) return;
    this.#remove(id);
    // A start still in flight ends its own session when it resolves (see `startIn`).
    if (tab.sessionId !== null) await this.#end(tab.sessionId);
  }

  move(from: number, to: number): void {
    this.#state = moved(this.#state, from, to);
  }

  async popOut(id: TabId): Promise<void> {
    const tab = this.#find(id);
    if (this.#popOut === undefined || tab === undefined || tab.protocol === 'WEB') return;
    await this.#popOut({ deviceId: tab.deviceId, deviceName: tab.deviceName, protocol: tab.protocol });
    // A tab closed while the window opened has already ended its session (see `close`).
    const sessionId = this.#find(id)?.sessionId;
    if (sessionId === undefined) return;
    this.#remove(id);
    if (sessionId !== null) await this.#end(sessionId);
  }

  #isLatest(id: TabId, start: number): boolean {
    return this.#find(id) !== undefined && this.#latestStart.get(id) === start;
  }

  /**
   * One start in tab `id` through the launch `pick` gives for the tab (`undefined`: none for this view). The tab
   * gives up the attempt's session when its tunnel ends (K4 §5).
   */
  async #begin<Tunnel extends { close(): void }>(
    id: TabId,
    pick: (tab: Tab) => Launch<Tunnel> | undefined,
  ): Promise<ViewStartResult<Tunnel>> {
    const tab = this.#find(id);
    if (tab === undefined) return TAB_CLOSED;
    const launch = pick(tab);
    if (launch === undefined) return OTHER_VIEW;
    this.#starts += 1;
    const mine = this.#starts;
    this.#latestStart.set(id, mine);
    if (tab.sessionId !== null) {
      const failure = await this.#endPrevious(id, tab.sessionId);
      if (failure !== undefined) return { ok: false, error: failure };
      if (!this.#isLatest(id, mine)) return TAB_CLOSED;
    }
    const attempt: Attempt = { sessionId: undefined, ended: false };
    const result = await launch(() => {
      this.#ended(id, attempt);
    });
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
  }

  /** The tunnel of `attempt` ended: the tab gives up its session, so the view ends it (K4 §5). */
  #ended(id: TabId, attempt: Attempt): void {
    attempt.ended = true;
    if (attempt.sessionId !== undefined && this.#find(id)?.sessionId === attempt.sessionId) {
      this.#state = withSession(this.#state, id, null);
    }
  }

  /**
   * A new start ends the tab's live session once before it asks for another (D-5). The tab gives the id up first,
   * so a close or another start meanwhile cannot end it again; on a failure it takes the id back. If a newer start
   * or a close has replaced the tab's id by then, the failed session is left to the hub, which ends it when its
   * tunnel closes (K4 §5): the view that owned it closed that tunnel before starting again.
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
