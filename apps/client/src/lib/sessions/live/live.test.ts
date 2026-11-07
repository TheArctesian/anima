// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { fakeHub } from '../../devices/hub.fixture';
import { endTransitionsAtOnce } from '../../devices/transitions.fixture';
import LiveSessions from './LiveSessions.svelte';

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

it('RA-034: the control shows the words, the live count and a name that includes the count', async () => {
  await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
  expect(control().querySelector('.words')?.textContent).toBe('Live sessions');
  await vi.waitFor(() => {
    expect(control().querySelector('.count')?.textContent).toBe('2');
  });
  expect(control().getAttribute('aria-label')).toBe('Live sessions, 2');
});

it('RA-034: the popover opens with a row per session, newest first, and closes with the control', async () => {
  await show({ LiveSessions: ANSWER(DESK, LAPTOP) });
  expect(control().getAttribute('aria-expanded')).toBe('false');
  await open();
  expect(control().getAttribute('aria-expanded')).toBe('true');
  expect(rows().map((row) => row.querySelector('strong')?.textContent)).toEqual(['laptop', 'desk']);
  expect(rows()[0]?.textContent).toContain('RDP');
  expect(rows()[0]?.textContent).toContain('ada');
  expect(rows()[0]?.textContent).toContain('100.64.0.2');
  control().click();
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
});

it('RA-034: with no live session the popover says so', async () => {
  await show({ LiveSessions: ANSWER() });
  await open();
  expect(panel()?.textContent).toContain('No live sessions.');
});

it('RA-034: Escape closes the popover and returns focus to the control', async () => {
  await show({ LiveSessions: ANSWER(DESK) });
  await open();
  stopButton('desk').focus();
  stopButton('desk').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
  expect(document.activeElement).toBe(control());
});

it('RA-034: a press outside closes the popover and a press inside does not', async () => {
  await show({ LiveSessions: ANSWER(DESK) });
  await open();
  panel()?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  expect(panel()).not.toBeNull();
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
});

it('RA-034: Stop ends the session and its row leaves', async () => {
  const fake = await show({ LiveSessions: ANSWER(DESK, LAPTOP), EndSession: { id: '1' } });
  await open();
  stopButton('desk').click();
  flushSync();
  await vi.waitFor(() => {
    expect(rows()).toHaveLength(1);
  });
  expect(fake.requests.find((request) => request.operation === 'EndSession')?.variables).toEqual({ id: '1' });
  expect(control().querySelector('.count')?.textContent).toBe('1');
});

it('RA-034: a failed Stop keeps the row and shows fixed text, not the hub text', async () => {
  await show({ LiveSessions: ANSWER(DESK), EndSession: REFUSED });
  await open();
  stopButton('desk').click();
  flushSync();
  await vi.waitFor(() => {
    expect(panel()?.textContent).toContain('The session could not be stopped.');
  });
  expect(rows()).toHaveLength(1);
  expect(panel()?.textContent).not.toContain('secret hub text');
  expect(stopButton('desk').disabled).toBe(false);
});

it('RA-034: Stop shows busy while the hub has not answered', async () => {
  const fake = fakeHub({ LiveSessions: ANSWER(DESK) });
  const never = new Promise<never>(() => undefined);
  const request = fake.hub.request.bind(fake.hub);
  fake.hub.request = ((document: never, variables: never) =>
    (document as { definitions: { name?: { value: string } }[] }).definitions[0]?.name?.value === 'EndSession'
      ? never
      : request(document, variables)) as typeof fake.hub.request;
  app = mount(LiveSessions, { target: document.body, props: { hub: fake.hub } });
  await vi.waitFor(() => {
    expect(control().querySelector('.count')?.textContent).toBe('1');
  });
  await open();
  stopButton('desk').click();
  flushSync();
  expect(stopButton('desk').disabled).toBe(true);
  expect(stopButton('desk').getAttribute('aria-busy')).toBe('true');
});

it('RA-034: a pushed start or end changes the list and the count', async () => {
  const fake = await show({ LiveSessions: ANSWER(DESK) });
  fake.push('LiveSessionChanged', { sessionChanged: { session: { ...LAPTOP, endedAt: null, endReason: null } } });
  flushSync();
  expect(control().querySelector('.count')?.textContent).toBe('2');
  fake.push('LiveSessionChanged', { sessionChanged: { session: { ...DESK, endedAt: '2026-11-04T12:00:00Z', endReason: 'OPERATOR' } } });
  flushSync();
  expect(control().querySelector('.count')?.textContent).toBe('1');
});

it('D-6: a subscription error keeps the list and shows no failure; reopening queries again', async () => {
  const answers: Record<string, unknown> = { LiveSessions: ANSWER(DESK) };
  const fake = await show(answers);
  fake.fail('LiveSessionChanged', { code: 'INTERNAL', message: 'Unknown field "sessionChanged"' });
  flushSync();
  await open();
  expect(control().querySelector('.count')?.textContent).toBe('1');
  expect(document.querySelector('[role="alert"]')).toBeNull();
  const before = fake.requests.filter((request) => request.operation === 'LiveSessions').length;
  control().click();
  flushSync();
  await vi.waitFor(() => {
    expect(panel()).toBeNull();
  });
  answers.LiveSessions = ANSWER(DESK, LAPTOP);
  await open();
  await vi.waitFor(() => {
    expect(control().querySelector('.count')?.textContent).toBe('2');
  });
  expect(fake.requests.filter((request) => request.operation === 'LiveSessions').length).toBe(before + 1);
});

it('RA-034: a failed query shows fixed text and keeps the last list', async () => {
  const answers: Record<string, unknown> = { LiveSessions: ANSWER(DESK) };
  await show(answers);
  answers.LiveSessions = REFUSED;
  await open();
  await vi.waitFor(() => {
    expect(panel()?.textContent).toContain('The live sessions could not be loaded.');
  });
  expect(panel()?.textContent).not.toContain('secret hub text');
  expect(rows()).toHaveLength(1);
});
