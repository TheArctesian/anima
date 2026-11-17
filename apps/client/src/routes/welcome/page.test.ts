// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ carried: vi.fn(), desktop: true }));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/desktop/local-hub/shell', () => ({ localHubCarried: mocks.carried }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});

let app: ReturnType<typeof mount> | undefined;

async function open(): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
  return target;
}

/** Each choice: its title, its link (if it can be chosen) and its reason (if not). */
function choices(target: HTMLElement) {
  return [...target.querySelectorAll('li')].map((item) => ({
    title: item.querySelector('strong')?.textContent,
    href: item.querySelector('a:first-child')?.getAttribute('href') ?? null,
    reason: item.querySelector('.reason')?.textContent ?? null,
  }));
}

beforeEach(() => {
  mocks.carried.mockReset().mockResolvedValue(true);
  mocks.desktop = true;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('FRN-001, NAV-009, HMD-022: the desktop app shows three choices, each on; Sign in is the primary one', async () => {
  const target = await open();
  expect(target.querySelector('h1')?.textContent).toBe('Welcome to Anima.ing');
  expect(document.activeElement).toBe(target.querySelector('h1'));
  expect(choices(target)).toEqual([
    { title: 'Sign in', href: '/welcome/hosted', reason: null },
    { title: 'Join a hub', href: '/welcome/join', reason: null },
    { title: 'Create a hub on this computer', href: '/welcome/create', reason: null },
  ]);
  expect(target.querySelector('a.secondary')?.getAttribute('href')).toBe('/sign-in');
  expect([...target.querySelectorAll('li')].map((item) => item.classList.contains('primary'))).toEqual([true, false, false]);
});

it('FRN-001: a build that carries no hub programs keeps Create off with the reason', async () => {
  mocks.carried.mockResolvedValue(false);
  const target = await open();
  expect(choices(target)[2]).toEqual({ title: 'Create a hub on this computer', href: null, reason: 'This build does not carry the hub programs.' });
});

it('CLI-001: a browser does not ask the shell', async () => {
  mocks.desktop = false;
  await open();
  expect(mocks.carried).not.toHaveBeenCalled();
});
