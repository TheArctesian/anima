// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, ok } from '$lib/access/access.fixture';
import { parseHubAddress, type Hub } from '$lib/hub/hub';
import Page from '../+page.svelte';
import { boxes, continueWith, settle, type } from '../page.fixture';

// The sign-in page on this computer's hub address (F-10): nothing is sent until a program of this user listens there.

const mocks = vi.hoisted(() => ({ hub: undefined as unknown, loopbackProblem: vi.fn() }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', async (importOriginal) => ({ ...(await importOriginal<typeof import('$lib/desktop/token')>()), writeAccessToken: vi.fn() }));
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => DESKTOP_MAC };
});
vi.mock('$lib/access/navigation', () => ({ goToDevices: vi.fn(), reloadClient: vi.fn(), loadClientAt: vi.fn() }));
vi.mock('$lib/access/listener/check', () => ({ loopbackProblem: mocks.loopbackProblem, tokenFor: () => () => Promise.resolve(null) }));

const PROBLEM = { title: "Another program holds the hub's port", advice: 'Anima.ing sends nothing to it: no password and no token. Stop that program, then check again.' };
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

function open(): HTMLElement {
  const made = fakeHub({ Login: ok({ login: { token: 'tok-secret' } }) });
  // This computer's hub address, which the page checks (F-10).
  const local = parseHubAddress('http://127.0.0.1:8443');
  if (!local.ok) throw new Error('bad address');
  hub = { ...made, hub: { ...made.hub, address: local.value } as Hub };
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

function button(target: HTMLElement, name: string): HTMLButtonElement | undefined {
  return [...target.querySelectorAll('button')].find((each) => each.textContent.trim() === name);
}

const fetch = vi.fn();
const sockets = vi.fn();

beforeEach(() => {
  mocks.loopbackProblem.mockReset();
  fetch.mockReset();
  sockets.mockReset();
  vi.stubGlobal('fetch', fetch);
  vi.stubGlobal('WebSocket', sockets);
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
});

it('F-10: when another program holds the port, the page says so, Continue is off and nothing is sent', async () => {
  mocks.loopbackProblem.mockResolvedValue(PROBLEM);
  const target = open();
  await settle();
  expect(mocks.loopbackProblem).toHaveBeenCalledWith(hub.hub.address.origin, true);
  expect(target.querySelector('[role="alert"]')?.textContent).toContain(PROBLEM.title);
  expect(button(target, 'Continue')?.disabled).toBe(true);
  await continueWith(target, 'ann', 'hunter2-pw');
  expect(boxes(target)).toHaveLength(0);
  expect(hub.sent).toEqual([]);
});

it('F-10, M-A: a loopback address typed into the hub field is checked first; with another program there no request and no socket open', async () => {
  mocks.loopbackProblem.mockImplementation((origin: string) => Promise.resolve(origin === 'http://127.0.0.1:9000' ? PROBLEM : null));
  const target = open();
  await settle();
  button(target, 'Change')?.click();
  await settle();
  type(target, 'hub-address', 'http://127.0.0.1:9000');
  await continueWith(target, 'ann', 'hunter2-pw');
  expect(mocks.loopbackProblem).toHaveBeenLastCalledWith('http://127.0.0.1:9000', true);
  expect(target.querySelector('[role="alert"]')?.textContent).toContain(PROBLEM.title);
  expect(boxes(target)).toHaveLength(0);
  expect([fetch.mock.calls.length, sockets.mock.calls.length, hub.sent.length]).toEqual([0, 0, 0]);
});

it('F-10, M-B: while the first check is pending, Continue is off and nothing moves on', async () => {
  let release: (value: null) => void = () => undefined;
  mocks.loopbackProblem.mockReturnValue(new Promise((resolve) => (release = resolve)));
  const target = open();
  await settle();
  expect(target.textContent).not.toContain(PROBLEM.title);
  release(null);
  await settle();
  expect(button(target, 'Continue')?.disabled).toBe(false);
});

it('F-10, M-B: the check runs again right before the password goes; a refusal then sends nothing', async () => {
  mocks.loopbackProblem.mockResolvedValueOnce(null).mockResolvedValueOnce(null).mockResolvedValue(PROBLEM);
  const target = open();
  await settle();
  await continueWith(target, 'ann', 'hunter2-pw');
  const event = new Event('paste', { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', { value: { getData: () => '123456' } });
  boxes(target)[0]?.dispatchEvent(event);
  await settle();
  expect(mocks.loopbackProblem).toHaveBeenCalledTimes(3);
  expect(hub.sent).toEqual([]);
  expect(target.querySelector('[role="alert"]')?.textContent).toContain(PROBLEM.title);
});

it('F-10: Check again asks again; a safe answer clears the notice', async () => {
  mocks.loopbackProblem.mockResolvedValueOnce(PROBLEM).mockResolvedValueOnce(null);
  const target = open();
  await settle();
  button(target, 'Check again')?.click();
  await settle();
  expect(mocks.loopbackProblem).toHaveBeenLastCalledWith(hub.hub.address.origin, true);
  expect(target.textContent).not.toContain(PROBLEM.title);
  expect(button(target, 'Continue')?.disabled).toBe(false);
});
