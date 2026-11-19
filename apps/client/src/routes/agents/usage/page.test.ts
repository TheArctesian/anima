// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeUsageHub, type FakeUsageHub } from '$lib/agent-usage/agent-usage.fixture';
import { installAnimations } from '$lib/forms/animations.fixture';
import { markHosted } from '$lib/hosted/mark';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ search: '', goto: vi.fn(() => Promise.resolve()), hub: undefined as unknown }));

vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('$app/state', () => ({ page: { get url() { return new URL(`https://hub.test/agents/usage${mocks.search}`); } } }));
vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

/** The origin of a hub that the application reaches through its link. */
const HUB = 'http://127.0.0.1:41001';

let app: ReturnType<typeof mount> | undefined;
let hub: FakeUsageHub;

beforeEach(() => {
  installAnimations();
  hub = fakeUsageHub();
  mocks.hub = Object.assign(hub, { address: { origin: HUB } });
  localStorage.clear();
  mocks.search = '';
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  mocks.goto.mockClear();
});

async function open(search: string): Promise<void> {
  mocks.search = search;
  app = mount(Page, { target: document.body });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

it('AIS-020: the page reads the use with the choices of its address', async () => {
  await open('?days=7&by=MODEL&source=CODEX&device=d-mac');
  const asked = hub.asked[0] as { filter: Record<string, string>; by: string[] };
  expect(asked.by).toEqual(['MODEL']);
  expect(asked.filter).toMatchObject({ source: 'CODEX', deviceId: 'd-mac' });
  expect(document.querySelector('.summary')?.textContent).toContain('17 sessions');
});

it('AIS-020: with no choices in the address the page reads the last 30 days by device and program', async () => {
  await open('');
  const asked = hub.asked[0] as { filter: Record<string, string>; by: string[] };
  expect(asked.by).toEqual(['DEVICE', 'SOURCE']);
  const days = (Date.parse(`${asked.filter['lastDay'] ?? ''}T00:00:00Z`) - Date.parse(`${asked.filter['firstDay'] ?? ''}T00:00:00Z`)) / 86_400_000;
  expect(days).toBe(29);
});

it('a choice goes to the address and keeps the others', async () => {
  await open('?by=MODEL');
  document.querySelector<HTMLElement>('[role="radio"][data-value="90"]')?.click();
  expect(mocks.goto).toHaveBeenCalledWith('/agents/usage?days=90&by=MODEL', { replaceState: true, keepFocus: true, noScroll: true });
});

it('DEF-002: the hub that this client marked as hosted gets the empty state with no link; a different hub gets Open Devices', async () => {
  hub.answer = { ...hub.answer, rows: [], devices: [] };
  hub.everyone = [];
  await open('');
  expect(document.querySelector('a')?.textContent).toBe('Open Devices');
  if (app !== undefined) await unmount(app);
  markHosted(localStorage, HUB);
  await open('');
  expect(document.querySelector('a')).toBeNull();
  expect(document.body.textContent).toContain('A daemon cannot connect to a hub that Anima.ing hosts yet');
});
