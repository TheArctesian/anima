// Test support: the root layout with a fake hub behind the session store, in jsdom.
// The test files mock `$lib/sessions/sessions.svelte` so `createSessions` uses `fake.port`.
import { createRawSnippet, flushSync, mount, unmount, type Component } from 'svelte';
import { vi } from 'vitest';
import type { PopOutPort, SessionPort, Sessions } from '../sessions.svelte';

export const fake = {
  port: undefined as unknown as SessionPort,
  popOut: undefined as PopOutPort | undefined,
  sessions: undefined as unknown as Sessions,
};

/** Every keyframe list Svelte's transitions asked for, in order. */
export const keyframes: Keyframe[][] = [];

let app: ReturnType<typeof mount> | undefined;

/** jsdom has no Web Animations; this one finishes at once so outros complete. */
function finishingAnimate(frames: Keyframe[]) {
  keyframes.push(frames);
  return {
    cancel: () => undefined,
    currentTime: 0,
    playState: 'finished',
    effect: null,
    set onfinish(done: () => void) {
      queueMicrotask(done);
    },
  };
}

const stubs: Record<string, unknown> = {
  animate: finishingAnimate,
  setPointerCapture: () => undefined,
  releasePointerCapture: () => undefined,
  hasPointerCapture: () => true,
};

/** A fresh fake hub whose starts give `session-1`, `session-2`, ...; and the jsdom stubs. */
export function prepare(): void {
  let count = 0;
  fake.port = {
    start: vi.fn(() => {
      count += 1;
      return Promise.resolve({ sessionId: `session-${String(count)}` });
    }),
    end: vi.fn(() => Promise.resolve()),
  };
  keyframes.length = 0;
  for (const [name, value] of Object.entries(stubs)) {
    Object.defineProperty(Element.prototype, name, { value, configurable: true });
  }
}

export async function cleanup(): Promise<void> {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  for (const name of Object.keys(stubs)) Reflect.deleteProperty(Element.prototype, name);
}

export async function settle(): Promise<void> {
  for (let tick = 0; tick < 6; tick += 1) {
    await Promise.resolve();
    flushSync();
  }
}

/** Mounts `layout` with a stand-in page; `popOut` is the window port, if any. */
export function show(layout: Component<{ children: ReturnType<typeof createRawSnippet> }>, popOut?: PopOutPort): Sessions {
  fake.popOut = popOut;
  const children = createRawSnippet(() => ({ render: () => '<p id="page">page</p>' }));
  app = mount(layout, { target: document.body, props: { children } });
  flushSync();
  return fake.sessions;
}

export const request = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH', origin: null } as const;

export const tabs = () => [...document.querySelectorAll<HTMLElement>('[role="tab"]')];

export const visiblePanels = () =>
  [...document.querySelectorAll('[role="tabpanel"]')].filter((panel) => !panel.closest('[hidden]'));

/** Opens a tab for `request`'s device and starts its session. */
export async function openStarted(sessions: Sessions, origin: DOMRect | null = null): Promise<string> {
  const id = sessions.openSession({ ...request, origin });
  await sessions.start(id, { widthPx: 800, heightPx: 600 });
  await settle();
  return id;
}
