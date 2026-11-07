// Test support: mounts `RemoteView` in jsdom on the real Guacamole tunnel over `FakeTunnelSocket`, with a fake
// sessions API that answers the start, the end and the transfer policy as K1 and K4 describe. It reuses the
// browser fakes of the SSH view fixture (ResizeObserver, system clipboard, Web Animations).
import { flushSync, mount, unmount } from 'svelte';
import type { HubResult } from '$lib/hub/hub';
import type { ConnectionTestInput, EndedSession, RemoteProtocol, SessionDisplay, SessionsApi, SessionTicket, SessionTransfer } from '../../api';
import type { SessionEndReason, SessionEnding } from '../../ending';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../../tunnel/fake/socket.fixture';
import type { SessionTunnel } from '../../tunnel/guacamole/tunnel';
import { CERTIFICATE } from '../../fake/hub.fixture';
import { sessionStart } from '../start';
import { FakeResizeObserver, TOKEN_MARKER, settle } from '../view.fixture.svelte';
import RemoteView from './RemoteView.svelte';

type Client = SessionTunnel['client'];

/** What the fake hub answers; tests change it between steps. */
export interface RemoteAnswers {
  endReason: SessionEndReason | null;
  /** The device's policy, or `unread` for a hub that cannot answer it (D-3). */
  transfer: SessionTransfer | 'unread';
  /** A refusal of `startSession`, by K7 code. */
  refuse: 'CONFLICT' | undefined;
  /** While set, the policy read waits for it, so the start has not answered yet. */
  policyGate: Promise<void> | undefined;
  /** While set, `setCredential` waits for it. */
  credentialGate: Promise<void> | undefined;
  /** `startSession` says it sent a Wake-on-LAN packet. */
  wakeSent: boolean;
}

/** Every call the fake API heard, in order per method. */
export interface RemoteCalls {
  startSession: [string, string, SessionDisplay][];
  endSession: string[];
  testConnection: ConnectionTestInput[];
  acceptServerCertificate: [string, string][];
  setCredential: [string, string, string][];
}

type RemoteApi = Pick<
  SessionsApi,
  'startSession' | 'endSession' | 'transferPolicy' | 'testConnection' | 'acceptServerCertificate' | 'setCredential'
>;

const PHASE_ONE_TEST = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };

function remoteApi() {
  const calls: RemoteCalls = { startSession: [], endSession: [], testConnection: [], acceptServerCertificate: [], setCredential: [] };
  const answers: RemoteAnswers = { endReason: null, transfer: { copy: true, paste: true }, refuse: undefined, policyGate: undefined, credentialGate: undefined, wakeSent: false };
  let issued = 0;
  const api: RemoteApi = {
    startSession: (deviceId, protocol, display) => {
      calls.startSession.push([deviceId, protocol, display]);
      if (answers.refuse !== undefined) return Promise.resolve({ ok: false, error: { code: answers.refuse, message: 'HUB-TEXT-MARKER' } });
      issued += 1;
      const n = String(issued);
      const ticket: SessionTicket = { sessionId: `session-${n}`, token: `${TOKEN_MARKER}-${n}`, tunnelPath: '/tunnel', expiresAt: '2026-11-04T10:00:30Z', wakeSent: answers.wakeSent };
      return Promise.resolve({ ok: true, value: ticket });
    },
    endSession: (id) => {
      calls.endSession.push(id);
      const ended: EndedSession = {
        id,
        deviceId: 'device-1',
        deviceName: 'desk',
        protocol: 'RDP',
        startedAt: '2026-11-04T10:00:00Z',
        endedAt: '2026-11-04T10:05:00Z',
        endReason: answers.endReason,
      };
      return Promise.resolve({ ok: true, value: ended });
    },
    testConnection: (input) => {
      calls.testConnection.push(input);
      return Promise.resolve({ ok: true, value: { ...PHASE_ONE_TEST, serverCertificate: CERTIFICATE, certificateChanged: true } });
    },
    acceptServerCertificate: (deviceId, fingerprint) => {
      calls.acceptServerCertificate.push([deviceId, fingerprint]);
      return Promise.resolve({ ok: true, value: CERTIFICATE });
    },
    setCredential: async (protocol, deviceId, password) => {
      calls.setCredential.push([protocol, deviceId, password]);
      await answers.credentialGate;
      return { ok: true, value: { credentialStored: true } };
    },
    transferPolicy: async () => {
      await answers.policyGate;
      const answer: HubResult<SessionTransfer> =
        answers.transfer === 'unread' ? { ok: false, error: { code: 'unreachable', message: 'No answer.' } } : { ok: true, value: answers.transfer };
      return answer;
    },
  };
  return { api, calls, answers };
}

/** The flags the parent passes, as state so that a test can change them on a mounted view. */
export const remoteFlags = $state({ visible: true, focused: true });

export const REMOTE_DEVICE = { id: 'device-1', name: 'desk' };

/** Mount a `RemoteView` for `REMOTE_DEVICE` over `protocol`, with `remoteFlags` as its flags. */
export function showRemote(protocol: RemoteProtocol = 'RDP') {
  const { api, calls, answers } = remoteApi();
  const heard = { ended: [] as SessionEnding[], clients: [] as Client[], gone: 0 };
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(RemoteView, {
    target,
    props: {
      device: REMOTE_DEVICE,
      protocol,
      start: sessionStart(api, REMOTE_DEVICE.id, 'https://hub.anima.ing', FAKE_TUNNEL_SOCKET).remote(protocol),
      api,
      get visible() {
        return remoteFlags.visible;
      },
      get focused() {
        return remoteFlags.focused;
      },
      onended: (ending: SessionEnding) => heard.ended.push(ending),
      onclient: (client: Client) => {
        heard.clients.push(client);
        return () => {
          heard.gone += 1;
        };
      },
    },
  });
  flushSync();
  return { calls, answers, heard, target, close: () => unmount(component) };
}

/** Measure the pane (which starts the session), let the hub answer, open the tunnel and draw the first frame. */
export async function connectRemote(width = 800, height = 600): Promise<FakeTunnelSocket> {
  FakeResizeObserver.resize(width, height);
  await settle();
  const socket = FakeTunnelSocket.latest();
  socket.open();
  socket.sync();
  flushSync();
  return socket;
}

/** The pane the keyboard listens to. */
export function remotePaneOf(target: HTMLElement): HTMLElement {
  const pane = target.querySelector<HTMLElement>('[role="application"]');
  if (pane === null) throw new Error('no remote pane');
  return pane;
}

/** The instructions the client sent with `opcode`, without their opcode. */
export function sentOf(socket: FakeTunnelSocket, opcode: string): string[][] {
  return socket.sentInstructions().filter(([name]) => name === opcode).map(([, ...args]) => args);
}

/** The key events the client sent, as `[pressed, keysym]`. */
export function keysOf(socket: FakeTunnelSocket): [number, number][] {
  return sentOf(socket, 'key').map(([keysym, pressed]) => [Number(pressed), Number(keysym)]);
}

/** A key event on `target` (the Guacamole keyboard reads `keyCode`, which the event constructor does not take). */
export function keyOn(target: EventTarget, type: 'keydown' | 'keyup', init: KeyboardEventInit & { keyCode: number }): KeyboardEvent {
  const event = new KeyboardEvent(type, { bubbles: true, cancelable: true, ...init });
  Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  target.dispatchEvent(event);
  return event;
}

/** A `paste` event on `target` carrying `text`. */
export function pasteOn(target: EventTarget, text: string): Event {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', { value: { getData: (type: string) => (type === 'text/plain' ? text : '') } });
  target.dispatchEvent(event);
  return event;
}

/** Open the Keys menu and choose the item named `label`. */
export function chooseKey(target: HTMLElement, label: string): HTMLButtonElement | undefined {
  target.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.click();
  flushSync();
  const item = [...document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((entry) => entry.querySelector('.label')?.textContent === label);
  item?.click();
  flushSync();
  return item;
}
