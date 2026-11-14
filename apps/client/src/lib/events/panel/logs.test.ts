// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import type { LogEvent } from '../api';
import LogsPanel from './LogsPanel.svelte';
import { runs } from '../../motion/engine.fixture';
import { NOW, event, eventsAnswer, settle } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

const devices = [
  { id: 'd1', name: 'Router' },
  { id: 'd2', name: 'NAS' },
];
let app: ReturnType<typeof mount> | undefined;
let fake: ReturnType<typeof fakeHub>;
let clock = NOW;

async function show(answer: unknown = eventsAnswer([event('e3', 1), event('e2', 2), event('e1', 3)])): Promise<void> {
  fake = fakeHub({ Events: answer });
  current.hub = fake.hub;
  clock = NOW;
  app = mount(LogsPanel, { target: document.body, props: { devices, now: () => clock } });
  await settle();
}

beforeEach(() => {
  preference.reduced = false;
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

const ids = () => [...document.querySelectorAll('.row .message')].map((node) => node.textContent);
const eventRequests = () => fake.requests.filter((request) => request.operation === 'Events');
const lastFilter = () => eventRequests().at(-1)?.variables as { filter: Record<string, unknown>; after?: string };
const button = (text: string) =>
  [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.includes(text));

function within(value: string): void {
  const radio = document.querySelector<HTMLElement>(`[role="radiogroup"][aria-label="Time"] [data-value="${value}"]`);
  if (!radio) throw new Error(`no ${value} window`);
  radio.click();
  flushSync();
}

function chip(severity: string): void {
  const control = [...document.querySelectorAll('[aria-label="Severity"] button')].find((each) => each.textContent.includes(severity));
  if (!(control instanceof HTMLElement)) throw new Error(`no ${severity} chip`);
  control.click();
  flushSync();
}

/** Opens the menu whose button says `menu` and checks or chooses `option`. */
function pick(menu: string, option: string): void {
  button(menu)?.click();
  flushSync();
  const label = [...document.querySelectorAll('.option')].find((row) => row.textContent.includes(option));
  const box = label?.querySelector('input');
  if (!box) throw new Error(`no ${option} option`);
  box.click();
  flushSync();
}

async function push(over: LogEvent): Promise<void> {
  fake.push('EventAdded', { eventAdded: over });
  await settle();
}

describe('LOG-009: the list', () => {
  it('shows the events as the hub sends them, newest first, with time, severity, source, device and message', async () => {
    await show(eventsAnswer([event('e2', 1, { severity: 'ERROR', source: 'HUB', deviceName: null }), event('e1', 2)]));
    expect(ids()).toEqual(['message e2', 'message e1']);
    const first = document.querySelector('.row');
    expect(first?.querySelector('.severity')?.textContent).toBe('Error');
    expect(first?.querySelector('.source')?.textContent).toBe('Hub');
    expect(first?.querySelector('.device')?.textContent).toBe('—');
    expect(first?.querySelector('time')?.getAttribute('datetime')).toBe(event('e2', 1).at);
    expect(document.querySelectorAll('.row .device')[1]?.textContent).toBe('Router');
  });

  it('has the title "Logs" and loads the last 24 hours at first', async () => {
    await show();
    expect(document.querySelector('h2')?.textContent).toBe('Logs');
    expect(eventRequests()).toHaveLength(1);
    expect(lastFilter().filter).toEqual({ since: new Date(NOW.getTime() - 24 * 3_600_000).toISOString() });
  });

});

describe('LOG-009: the filters', () => {
  it('sends the time filter', async () => {
    await show();
    within('1h');
    await settle();
    expect(lastFilter().filter).toEqual({ since: new Date(NOW.getTime() - 3_600_000).toISOString() });
    within('all');
    await settle();
    expect(lastFilter().filter).toEqual({});
  });

  it('sends the severity filter: every chip is on at first, and turning chips off narrows it', async () => {
    await show();
    expect([...document.querySelectorAll('[aria-label="Severity"] [aria-pressed="true"]')]).toHaveLength(3);
    chip('Info');
    await settle();
    expect(lastFilter().filter).toMatchObject({ severity: ['ERROR', 'WARNING'] });
    chip('Warning');
    await settle();
    expect(lastFilter().filter).toMatchObject({ severity: ['ERROR'] });
    chip('Error');
    await settle();
    expect(eventRequests()).toHaveLength(3);
    chip('Info');
    chip('Warning');
    await settle();
    expect(lastFilter().filter).not.toHaveProperty('severity');
  });

  it('sends the source filter', async () => {
    await show();
    pick('Source', 'Session');
    await settle();
    expect(lastFilter().filter).toMatchObject({ source: ['SESSION'] });
  });

  it('sends the device filter from the devices the page has, and takes it off again', async () => {
    await show();
    pick('All devices', 'NAS');
    await settle();
    expect(lastFilter().filter).toMatchObject({ deviceId: 'd2' });
    pick('NAS', 'All devices');
    await settle();
    expect(lastFilter().filter).not.toHaveProperty('deviceId');
  });

  it('does not show a live event that the filter does not match', async () => {
    await show();
    pick('All devices', 'NAS');
    await settle();
    await push(event('e9', 0, { deviceId: 'd1' }));
    expect(ids()).not.toContain('message e9');
    await push(event('e8', 0, { deviceId: 'd2' }));
    expect(ids()[0]).toBe('message e8');
  });

  it('shows an empty log as "No events"', async () => {
    await show(eventsAnswer([]));
    expect(document.querySelector('.none')?.textContent).toBe('No events');
  });
});

describe('LOG-010: one filter change', () => {
  it('makes exactly one request and one subscription', async () => {
    await show();
    within('1h');
    await settle();
    expect(eventRequests()).toHaveLength(2);
    expect(fake.subscriptions).toHaveLength(2);
    expect(fake.listeners('EventAdded')).toBe(1);
  });
});

describe('LOG-009: older events', () => {
  it('offers "Load older" only while the hub has another page, and sends the cursor', async () => {
    await show(eventsAnswer([event('e2', 1), event('e1', 2)], 'cursor-1'));
    expect(button('Load older')?.getAttribute('tabindex')).toBe('0');
    expect(button('Collapse')?.getAttribute('tabindex')).toBe('0');
    button('Load older')?.click();
    await settle();
    expect(lastFilter().after).toBe('cursor-1');
  });

  it('hides "Load older" when the hub has no other page', async () => {
    await show();
    expect(button('Load older')).toBeUndefined();
  });

  it('a full log of 500 rows offers no "Load older" although the hub has more', async () => {
    const full = Array.from({ length: 500 }, (_, index) => event(`f${String(index)}`, index));
    await show(eventsAnswer(full, 'more'));
    expect(document.querySelectorAll('.row')).toHaveLength(500);
    expect(button('Load older')).toBeUndefined();
  });
});

describe('LOG-010: live events', () => {
  it('puts a pushed event on top with no new request', async () => {
    await show();
    await push(event('e4', 0));
    expect(ids()[0]).toBe('message e4');
    expect(eventRequests()).toHaveLength(1);
  });

  it('listens from INFO at first and again with the lowest chosen severity after a filter change', async () => {
    await show();
    expect(fake.subscriptions).toEqual([{ operation: 'EventAdded', variables: { minSeverity: 'INFO' } }]);
    chip('Info');
    chip('Warning');
    await settle();
    expect(fake.subscriptions.at(-1)).toEqual({ operation: 'EventAdded', variables: { minSeverity: 'ERROR' } });
    expect(fake.listeners('EventAdded')).toBe(1);
  });

  it('judges each live event by the clock at that time', async () => {
    await show();
    within('1h');
    await settle();
    await push(event('e5', 30));
    expect(ids()).toContain('message e5');
    clock = new Date(NOW.getTime() + 2 * 3_600_000);
    await push(event('e6', 30));
    expect(ids()).not.toContain('message e6');
  });

  it('stops listening when the panel goes away', async () => {
    await show();
    expect(fake.listeners('EventAdded')).toBe(1);
    if (app) void unmount(app);
    app = undefined;
    expect(fake.listeners('EventAdded')).toBe(0);
  });

  it('says so when the live updates fail', async () => {
    await show();
    fake.fail('EventAdded', { code: 'unreachable', message: 'The hub went away.' });
    await settle();
    expect(document.querySelector('[role="status"]')?.textContent).toContain('The hub went away.');
  });
});

describe('Text safety and errors', () => {
  it('LOG-009: a message with markup renders as text', async () => {
    const message = '<img src=x onerror="alert(1)"> & <b>bold</b>';
    await show(eventsAnswer([event('e1', 1, { message })]));
    expect(document.querySelector('.message')?.textContent).toBe(message);
    expect(document.querySelector('img')).toBeNull();
    expect(document.querySelector('.message b')).toBeNull();
  });

  it('LOG-009: a pushed message with markup renders as text', async () => {
    await show();
    await push(event('e7', 0, { message: '<script>x()</script>' }));
    expect(ids()[0]).toBe('<script>x()</script>');
    expect(document.querySelector('.message script')).toBeNull();
  });

  it('LOG-009: a device name and a source with markup render as text', async () => {
    const source = '<b>src</b>' as LogEvent['source'];
    await show(eventsAnswer([event('e1', 1, { deviceName: '<i>name</i>', source })]));
    expect(document.querySelector('.device')?.textContent).toBe('<i>name</i>');
    expect(document.querySelector('.source')?.textContent).toBe('<b>src</b>');
    expect(document.querySelector('.row i, .row b')).toBeNull();
  });

  it('shows the K7 error text with a Retry that loads the first page again', async () => {
    await show({ code: 'unreachable', message: 'The hub did not answer.' });
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('The hub did not answer.');
    button('Retry')?.click();
    await settle();
    expect(eventRequests()).toHaveLength(2);
  });
});

describe('when live logs stop (NET-004, MOT-007)', () => {
  const stopped = { code: 'unreachable', message: 'The connection closed' } as const;

  it('shows the notice as a status that fades in, so the rows below never step by its height', async () => {
    await show();
    runs.length = 0;
    fake.fail('EventAdded', stopped);
    await settle();
    const notice = document.querySelector('.notice[role="status"]') as HTMLElement | null;
    expect(notice?.textContent).toBe('Live logs stopped: The connection closed');
    // Fades to 1 over duration-base: an instant mount has no such run.
    expect(runs.filter((run) => run.keyframes[0] === 0 && run.keyframes.at(-1) === 1)).toHaveLength(1);
  });

  it('moves the rows by spring from where they were, and never translates the fixed panel or its scroll region', async () => {
    await show();
    const stack = document.querySelector('.stack') as HTMLElement;
    const panel = document.querySelector('section.panel') as HTMLElement;
    const scroll = document.querySelector('.scroll') as HTMLElement;
    const panelTranslate = panel.style.translate;
    const HEIGHT = 56;
    // Every box is stubbed to move with the notice, so one that is wrongly measured would be translated.
    for (const box of [stack, panel, scroll]) {
      box.getBoundingClientRect = () => {
        const notice = document.querySelector('.notice[role="status"]');
        const inFlow = notice !== null && !notice.classList.contains('leaving');
        const [, y = 0] = box.style.translate.split(/\s+/).filter((part) => part !== '').map((part) => Number.parseFloat(part));
        return new DOMRect(0, 100 + (inFlow ? HEIGHT : 0) + (Number.isNaN(y) ? 0 : y), 100, 200);
      };
    }
    runs.length = 0;
    fake.fail('EventAdded', stopped);
    await settle();
    await settle();
    expect(stack.style.translate).toBe(`0px ${String(-HEIGHT)}px`);
    expect(runs.some((run) => run.options['stiffness'] !== undefined)).toBe(true);
    expect(panel.style.translate).toBe(panelTranslate);
    expect(scroll.style.translate).toBe('');
  });
});
