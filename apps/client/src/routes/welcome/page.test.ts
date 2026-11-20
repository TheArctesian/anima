// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ desktop: true }));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});

let app: ReturnType<typeof mount> | undefined;

function open(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

beforeEach(() => {
  mocks.desktop = true;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('FRN-001, NAV-009, HMD-022: the first screen offers sign-in and joining with an enrollment QR', () => {
  const target = open();
  expect(target.querySelector('h1')?.textContent).toBe('Welcome to Anima.ing');
  expect(document.activeElement).toBe(target.querySelector('h1'));
  const links = [...target.querySelectorAll('a')].map((link) => [link.textContent, link.getAttribute('href')]);
  expect(links).toEqual([
    ['Sign in', '/welcome/hosted'],
    ['Sign up', '/welcome/hosted'],
    ['Join a hub', '/welcome/hub'],
    ['Scan enrollment QR', '/welcome/join'],
  ]);
});

it('CLI-001: in a browser Sign in and Sign up are off and say why; Join a hub stays', () => {
  mocks.desktop = false;
  const target = open();
  expect(target.querySelector('.primary')?.getAttribute('aria-disabled')).toBe('true');
  expect(target.querySelector('.reason')?.textContent).toBe('Needs the Anima.ing app.');
  expect([...target.querySelectorAll('a')].map((link) => link.textContent)).toEqual(['Join a hub']);
});
