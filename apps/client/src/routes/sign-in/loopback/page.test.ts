// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, ok } from '$lib/access/access.fixture';
import type { Hub } from '$lib/hub/hub';
import Page from '../+page.svelte';
import { boxes, continueWith, settle } from '../page.fixture';

// The sign-in page on this computer's hub address (F-10): nothing is sent until a program of this user listens there.

const mocks = vi.hoisted(() => ({ hub: undefined as unknown, loopbackProblem: vi.fn(), forget: vi.fn() }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', async (importOriginal) => ({ ...(await importOriginal<typeof import('$lib/desktop/token')>()), writeAccessToken: vi.fn() }));
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => DESKTOP_MAC };
});
vi.mock('$lib/access/navigation', () => ({ goToDevices: vi.fn(), reloadClient: vi.fn(), loadClientAt: vi.fn() }));
vi.mock('$lib/access/listener/check', () => ({ loopbackProblem: mocks.loopbackProblem, hubListeners: { forget: mocks.forget } }));

const PROBLEM = { title: "Another program holds the hub's port", advice: 'Anima.ing sends nothing to it: no password and no token. Stop that program, then check again.' };
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

function open(): HTMLElement {
  hub = fakeHub({ Login: ok({ login: { token: 'tok-secret' } }) });
  mocks.hub = hub.hub satisfies Hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

function button(target: HTMLElement, name: string): HTMLButtonElement | undefined {
  return [...target.querySelectorAll('button')].find((each) => each.textContent.trim() === name);
}

beforeEach(() => {
  mocks.loopbackProblem.mockReset();
  mocks.forget.mockReset();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
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

it('F-10: Check again forgets the verdict and asks again; a safe answer clears the notice', async () => {
  mocks.loopbackProblem.mockResolvedValueOnce(PROBLEM).mockResolvedValueOnce(null);
  const target = open();
  await settle();
  button(target, 'Check again')?.click();
  await settle();
  expect(mocks.forget).toHaveBeenCalledWith(hub.hub.address.origin);
  expect(target.textContent).not.toContain(PROBLEM.title);
  expect(button(target, 'Continue')?.disabled).toBe(false);
});
