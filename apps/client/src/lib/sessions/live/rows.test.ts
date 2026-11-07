// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { fakeHub } from '../../devices/hub.fixture';
import { endTransitionsAtOnce } from '../../devices/transitions.fixture';
import LiveSessions from './LiveSessions.svelte';

const flipped = vi.hoisted(() => ({ calls: [] as ReadonlyMap<unknown, unknown>[] }));

vi.mock('../../motion/roles/flip', async (importOriginal) => {
  const real = await importOriginal<typeof import('../../motion/roles/flip')>();
  return { ...real, flip: (...args: Parameters<typeof real.flip>) => (flipped.calls.push(args[0]), real.flip(...args)) };
});

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

const node = (id: string, deviceName: string, startedAt: string) => ({
  id,
  deviceId: `device-${id}`,
  deviceName,
  protocol: 'RDP',
  operatorName: 'ada',
  startedAt,
  clientAddr: '100.64.0.2',
});
const ANSWER = (...nodes: ReturnType<typeof node>[]) => ({ sessions: { edges: nodes.map((one) => ({ node: one })) } });
const DESK = node('1', 'desk', '2026-11-04T10:00:00Z');
const LAPTOP = node('2', 'laptop', '2026-11-04T11:00:00Z');
const REFUSED = { code: 'INTERNAL', message: 'secret hub text' };

let app: ReturnType<typeof mount> | undefined;

async function show(answers: Record<string, unknown>) {
  const fake = fakeHub(answers);
  app = mount(LiveSessions, { target: document.body, props: { hub: fake.hub } });
  flushSync();
  await vi.waitFor(() => {
    expect(fake.requests.some((request) => request.operation === 'LiveSessions')).toBe(true);
  });
  await Promise.resolve();
  flushSync();
  return fake;
}

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

const control = () => document.querySelector<HTMLButtonElement>('.control') as HTMLButtonElement;
const panel = () => document.querySelector('.panel');
const rows = () => [...document.querySelectorAll('.row')];
const stopButton = (device: string) =>
  document.querySelector<HTMLButtonElement>(`button[aria-label="Stop session on ${device}"]`) as HTMLButtonElement;

async function open(): Promise<void> {
  control().click();
  flushSync();
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}

const press = (target: Element, key: string, shiftKey = false) => {
  target.dispatchEvent(new KeyboardEvent('keydown', { key, shiftKey, bubbles: true, cancelable: true }));
  flushSync();
};

it('RA-034, C-3: opening moves focus to the first Stop, or to the panel when the list is empty', async () => {
  await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
  await open();
  expect(document.activeElement).toBe(stopButton('laptop'));
  control().click();
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
  document.body.innerHTML = '';
  await unmount(app as ReturnType<typeof mount>);
  await show({ LiveSessions: ANSWER() });
  await open();
  expect(document.activeElement).toBe(panel());
});

it('RA-034, C-3: Tab from the control enters the panel, cycles the Stops, and Shift+Tab from the first returns to the control', async () => {
  await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
  await open();
  control().focus();
  press(control(), 'Tab');
  expect(document.activeElement).toBe(stopButton('laptop'));
  press(stopButton('laptop'), 'Tab');
  expect(document.activeElement).toBe(stopButton('desk'));
  press(stopButton('desk'), 'Tab');
  expect(document.activeElement).toBe(stopButton('laptop'));
  press(stopButton('laptop'), 'Tab', true);
  expect(document.activeElement).toBe(control());
  expect(panel()).not.toBeNull();
});

it('MOT-007, RA-034: a refused Stop reserves its line in every row first, so the row gains no element and the line fills the held place', async () => {
  await show({ LiveSessions: ANSWER(DESK), EndSession: REFUSED });
  await open();
  const facts = () => document.querySelector('.facts') as HTMLElement;
  const before = [...facts().children].map((child) => child.className);
  const reserved = facts().querySelector('.slot .sizer');
  expect(reserved?.getAttribute('aria-hidden')).toBe('true');
  expect(reserved?.textContent).toContain('The session could not be stopped.');
  stopButton('desk').click();
  await vi.waitFor(() => {
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('The session could not be stopped.');
  });
  expect([...facts().children].map((child) => child.className)).toEqual(before);
  expect(document.querySelector('[role="alert"]')?.parentElement).toBe(reserved?.parentElement);
  // A retry clears the line and shows it again in the same place.
  stopButton('desk').click();
  await vi.waitFor(() => {
    expect(document.querySelector('[role="alert"]')).not.toBeNull();
  });
  expect([...facts().children].map((child) => child.className)).toEqual(before);
});

/** Holds every transition until `finish`, and lays the rows out 64 px apart, as the list does in a browser. */
function holdTransitions() {
  const held: { onfinish: (() => void) | null }[] = [];
  const animate = Element.prototype.animate;
  const rect = Element.prototype.getBoundingClientRect;
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: function hold() {
      const playback = { onfinish: null, cancel: () => undefined, finish: () => undefined, currentTime: 0 };
      held.push(playback);
      return playback;
    },
  });
  Element.prototype.getBoundingClientRect = function place(this: Element) {
    const top = this.classList.contains('row') ? 64 * rows().indexOf(this) : 0;
    return { top, left: 0, right: 0, bottom: top, width: 0, height: 0, x: 0, y: top, toJSON: () => ({}) };
  };
  return {
    finish: () => {
      // A transition plays a delay animation, then its main one: finish until none is left.
      for (let playback = held.shift(); playback; playback = held.shift()) playback.onfinish?.();
    },
    restore: () => {
      Object.defineProperty(Element.prototype, 'animate', { configurable: true, value: animate });
      Element.prototype.getBoundingClientRect = rect;
    },
  };
}

const ended = (one: ReturnType<typeof node>) => ({
  sessionChanged: { session: { ...one, endedAt: '2026-11-04T12:00:00Z', endReason: 'OPERATOR' } },
});
const started = (one: ReturnType<typeof node>) => ({ sessionChanged: { session: { ...one, endedAt: null, endReason: null } } });
const names = () => rows().map((row) => row.querySelector('strong')?.textContent);

it('MOT-007: a leaving row keeps its place while it fades, then is removed', async () => {
  const motion = holdTransitions();
  try {
    const fake = await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
    await open();
    fake.push('LiveSessionChanged', ended(LAPTOP));
    flushSync();
    expect(names()).toEqual(['laptop', 'desk']);
    motion.finish();
    await vi.waitFor(() => {
      expect(names()).toEqual(['desk']);
    });
  } finally {
    motion.restore();
  }
});

it('MOT-007, MOT-002: when a leaving row has faded the rows after it glide into the gap by FLIP', async () => {
  const motion = holdTransitions();
  try {
    const fake = await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
    await open();
    const desk = rows()[1];
    flipped.calls.length = 0;
    fake.push('LiveSessionChanged', ended(LAPTOP));
    flushSync();
    expect(flipped.calls).toHaveLength(0);
    motion.finish();
    await vi.waitFor(() => {
      expect(flipped.calls.some((places) => places.has(desk as Element))).toBe(true);
    });
  } finally {
    motion.restore();
  }
});

it('MOT-007, MOT-002: a second change while the first row is still fading leaves the right list', async () => {
  const motion = holdTransitions();
  try {
    const third = node('3', 'tablet', '2026-11-04T12:30:00Z');
    const fake = await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
    await open();
    fake.push('LiveSessionChanged', ended(LAPTOP));
    flushSync();
    fake.push('LiveSessionChanged', started(third));
    flushSync();
    fake.push('LiveSessionChanged', ended(DESK));
    flushSync();
    motion.finish();
    await vi.waitFor(() => {
      expect(names()).toEqual(['tablet']);
    });
    expect(control().querySelector('.count')?.textContent).toBe('1');
  } finally {
    motion.restore();
  }
});

it('RA-034, C-3: when another client ends the session whose Stop has focus, focus moves to the next Stop, else the panel', async () => {
  const fake = await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
  await open();
  stopButton('laptop').focus();
  fake.push('LiveSessionChanged', ended(LAPTOP));
  flushSync();
  await vi.waitFor(() => {
    expect(document.activeElement).toBe(stopButton('desk'));
  });
  fake.push('LiveSessionChanged', ended(DESK));
  flushSync();
  await vi.waitFor(() => {
    expect(document.activeElement).toBe(panel());
  });
});
