// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import HubLinkRow from './HubLinkRow.svelte';

const mocks = vi.hoisted(() => ({ readLink: vi.fn(), leaveLink: vi.fn(), clear: vi.fn(), forget: vi.fn(), loadClientAt: vi.fn() }));
vi.mock('$lib/desktop/link/shell', () => ({ readLink: mocks.readLink, leaveLink: mocks.leaveLink }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: mocks.clear }));
vi.mock('$lib/access/listener/check', () => ({ forgetListener: mocks.forget }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: mocks.loadClientAt }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const LINK = 'http://127.0.0.1:41000';
let app: ReturnType<typeof mount> | undefined;

async function settle(): Promise<void> {
  for (let round = 0; round < 3; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

async function click(label: string): Promise<void> {
  const found = [...document.querySelectorAll('button')].filter((button) => button.textContent.trim() === label).at(-1);
  if (found === undefined) throw new Error(`no button ${label}`);
  found.click();
  await settle();
}

beforeEach(async () => {
  installAnimations();
  mocks.readLink.mockReset().mockResolvedValue({ ok: true, value: { joined: true, state: 'up', port: 41_000, path: 'direct' } });
  mocks.leaveLink.mockReset().mockResolvedValue({ ok: true, value: null });
  mocks.clear.mockReset().mockResolvedValue(undefined);
  mocks.forget.mockReset();
  mocks.loadClientAt.mockReset();
  localStorage.setItem('anima.hubAddress', LINK);
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(HubLinkRow, { target });
  await settle();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
});

it('FRN-005, M-C: Leave forgets the listener answer, the token and the saved address of the link, then shows the first run', async () => {
  await click('Leave this hub');
  await click('Leave this hub');
  expect(mocks.leaveLink).toHaveBeenCalledOnce();
  expect(mocks.forget).toHaveBeenCalledWith(LINK);
  expect(mocks.clear.mock.calls[0]?.[0]).toMatchObject({ origin: LINK });
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.loadClientAt).toHaveBeenCalledWith('/welcome');
});

it('FRN-005, S3: a token the key store keeps is shown, the address stays until Retry clears it, and the link is left once', async () => {
  mocks.clear.mockRejectedValueOnce(new Error('denied'));
  await click('Leave this hub');
  await click('Leave this hub');
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('kept the access token');
  expect(localStorage.getItem('anima.hubAddress')).toBe(LINK);
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
  await click('Retry');
  expect(mocks.leaveLink).toHaveBeenCalledOnce();
  expect(mocks.clear).toHaveBeenCalledTimes(2);
  expect(localStorage.getItem('anima.hubAddress')).toBeNull();
  expect(mocks.loadClientAt).toHaveBeenCalledWith('/welcome');
});
