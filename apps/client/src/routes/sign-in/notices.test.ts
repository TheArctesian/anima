// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { failure, fakeHub } from '$lib/access/access.fixture';
import type { Hub, HubStatus } from '$lib/hub/hub';
import Page from './+page.svelte';
import { input, pasteInto, problemOf, submit, type } from './page.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => DESKTOP_MAC };
});
vi.mock('$lib/access/navigation', () => ({ goToDevices: vi.fn(), reloadClient: vi.fn(), loadClientAt: vi.fn() }));

let app: ReturnType<typeof mount> | undefined;

function open(answers: Parameters<typeof fakeHub>[0], status?: HubStatus): HTMLElement {
  mocks.hub = fakeHub(answers, status).hub satisfies Hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

beforeEach(() => {
  vi.spyOn(console, 'warn').mockImplementation(() => undefined);
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

it('NAV-009, NET-004, UI-030: an unreachable hub shows a notice at the top of the card with the cause, the next step and the detail', () => {
  const target = open({}, { connection: 'unreachable', lastContactAt: null });
  const notice = target.querySelector('.card [role="status"]');
  expect(notice?.textContent).toContain("Can't reach hub.example.org");
  expect(notice?.textContent).toContain('Check the VPN client, then try again.');
  expect(notice?.textContent).toContain('https://hub.example.org');
  expect(notice?.textContent).toContain('No data received yet');
  expect(target.querySelector('.card')?.firstElementChild).toBe(notice);
});

it('NAV-009: a hub that answers shows no status text at all', () => {
  const target = open({}, { connection: 'connected', lastContactAt: new Date() });
  expect(target.querySelector('[role="status"]')).toBeNull();
  expect(target.textContent).not.toContain('Connected');
  expect(target.textContent).not.toContain("Can't reach");
});

it('UI-030: the form-level message is a notice inside the card, above the Continue button', async () => {
  const target = open({ Login: failure('UNAUTHENTICATED') });
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  await pasteInto(target, 0, '123456');
  const alert = target.querySelector('.card [role="alert"]');
  expect(alert?.textContent).toBe('The user name, password or code is wrong.');
  const button = target.querySelector('.card button[type="submit"]');
  expect(alert?.closest('.notice')?.compareDocumentPosition(button as Node)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
});

it('UI-030: a field error is named by aria-describedby and marks the field aria-invalid; a good field has neither error', async () => {
  const target = open({});
  type(target, 'username', 'ann');
  await submit(target, 0);
  const password = input(target, 'password');
  expect(password.getAttribute('aria-invalid')).toBe('true');
  const described = target.querySelector(`#${password.getAttribute('aria-describedby') ?? 'none'}`);
  expect(described?.hasAttribute('hidden')).toBe(false);
  expect(problemOf(target, 'password')).toBe('Type your password.');
  expect(problemOf(target, 'username')).toBe('');
  expect(target.querySelector('#username-problem')?.hasAttribute('hidden')).toBe(true);
});

it('NAV-005: tab order follows the page: user name, password, Continue, Change', () => {
  const target = open({});
  const order = [...target.querySelectorAll<HTMLElement>('input, button')].map((element) => element.id || element.textContent.trim());
  expect(order).toEqual(['username', 'password', 'Continue', 'Change']);
});

it('NAV-009: focus starts in User name', () => {
  const target = open({});
  expect(document.activeElement).toBe(target.querySelector('#username'));
});

it('UI-030: when the hub did not answer and the notice already says so, the attempt adds no second message', async () => {
  const target = open({ Login: failure('unreachable', 'The hub did not answer. Check the VPN client.') }, { connection: 'unreachable', lastContactAt: null });
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  await pasteInto(target, 0, '123456');
  expect(target.querySelector('.card [role="status"]')).not.toBeNull();
  expect(target.querySelector('.card [role="alert"]')).toBeNull();
});

it('UI-030: when the hub did not answer but no notice shows yet, the attempt says so itself', async () => {
  const target = open({ Login: failure('unreachable', 'The hub did not answer. Check the VPN client.') }, { connection: 'connected', lastContactAt: new Date() });
  type(target, 'username', 'ann');
  type(target, 'password', 'pw');
  await submit(target, 0);
  await pasteInto(target, 0, '123456');
  expect(target.querySelector('.card [role="alert"]')?.textContent).toBe('The hub did not answer. Check the VPN client.');
});
