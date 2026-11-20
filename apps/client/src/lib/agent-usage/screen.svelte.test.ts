// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { agentUsageApi } from './api';
import { ANSWER, DEVICES, fakeUsageHub, sums, type FakeUsageHub } from './agent-usage.fixture';
import AgentUsageScreen from './AgentUsageScreen.svelte';
import { DEFAULT_QUERY, usageRequest, type UsageQuery } from './query';
import { UsageStore } from './store.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: FakeUsageHub;
let changes: UsageQuery[];
const NOW = new Date('2026-11-17T10:00:00Z');
const NO_USE = { ...ANSWER, days: [], models: [], devices: [], repositories: [], newest: [], total: sums(), everything: sums() };

beforeEach(() => {
  installAnimations();
  hub = fakeUsageHub();
  changes = [];
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(query: UsageQuery = DEFAULT_QUERY, hostedLine?: string): Promise<UsageStore> {
  const store = new UsageStore(agentUsageApi(hub).read);
  const reading = store.load(usageRequest(query, NOW));
  app = mount(AgentUsageScreen, {
    target: document.body,
    props: { store, query, hostedLine, now: () => NOW, onQueryChange: (next: UsageQuery) => changes.push(next), onRetry: () => void store.load(usageRequest(query, NOW)) },
  });
  flushSync();
  await reading;
  flushSync();
  return store;
}

const text = () => document.body.textContent;
const control = (label: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button.control')].find((button) => (button.getAttribute('aria-label') ?? button.textContent.trim()).startsWith(label));
const table = (label: string) => [...document.querySelectorAll('table')].find((each) => each.querySelector('caption')?.textContent === label);
const lines = (label: string) => [...(table(label)?.querySelectorAll('tbody tr') ?? [])];
const cells = (line: Element | undefined) => [...(line?.querySelectorAll('td') ?? [])].map((cell) => cell.textContent.trim());
const bars = () => [...document.querySelectorAll('ol.days li')];

it('AIS-020, AIS-021: the totals give the sessions, prompts, tool calls, all tokens and the estimate, which says it is not a bill', async () => {
  await open();
  const figures = [...document.querySelectorAll('.figures div')].map((figure) => [figure.querySelector('dt')?.textContent, figure.querySelector('dd')?.textContent]);
  expect(figures).toEqual([['Sessions', '17'], ['Prompts', '169'], ['Tool calls', '1330'], ['Tokens', '2.7M'], ['Cost, estimate', '$12.40']]);
  expect(text()).toContain('list prices of 1 Nov; not a bill. 800 tokens have no known price');
  expect(text()).toContain('Input 19.3k');
  expect(text()).toContain('Cache read 2.2M');
});

it('with every model priced the estimate names no tokens without a price', async () => {
  hub.answer = { ...ANSWER, total: { ...ANSWER.total, unpricedTokens: 0 } };
  await open();
  expect(text()).not.toContain('no known price');
});

it('AIS-020: there is one bar for each day, and each bar says its day and its value as text', async () => {
  await open();
  expect(bars()).toHaveLength(7);
  expect(bars().at(-1)?.textContent.trim()).toBe('2026-11-17: $5.00');
  expect(document.querySelector('ol.days')?.getAttribute('aria-label')).toBe('Estimated cost for each day (UTC)');
});

it('the bars count what the page counts', async () => {
  await open({ ...DEFAULT_QUERY, count: 'sessions' });
  expect(bars().at(-1)?.textContent.trim()).toBe('2026-11-17: 7 sessions');
  expect(document.querySelector('ol.days')?.getAttribute('aria-label')).toBe('Sessions for each day (UTC)');
});

it('AIS-021: a line for each program and model, with tokens in the units of the command and the estimate in dollars', async () => {
  await open();
  const [opus, codex] = lines('By program and model');
  expect(opus?.querySelector('th')?.textContent.replace(/\s+/g, ' ').trim()).toBe('Claude Code claude-opus-4');
  expect(cells(opus)).toEqual(['12', '140', '2.7M', '$9.30']);
  expect(cells(codex)).toEqual(['5', '29', '5.0k', '$3.10']);
});

it('a model with no price reads No price', async () => {
  hub.answer = { ...ANSWER, models: [{ ...sums({ sessions: 1, estimatedCostUsd: null }), source: 'CODEX', model: null }] };
  await open();
  const [line] = lines('By program and model');
  expect(line?.querySelector('th')?.textContent).toContain('No model');
  expect(cells(line).at(-1)).toBe('No price');
});

it('a picked program and model limits the bars to its days, and a second press shows all again', async () => {
  await open();
  const press = () => {
    lines('By program and model')[1]?.querySelector('button')?.click();
    flushSync();
  };
  press();
  expect(bars().at(-1)?.textContent.trim()).toBe('2026-11-17: $1.00');
  expect(document.querySelector('ol.days')?.getAttribute('aria-label')).toBe('Estimated cost for each day (UTC), Codex gpt-5-codex only');
  expect(lines('By program and model')[1]?.getAttribute('aria-selected')).toBe('true');
  expect(changes).toEqual([]);
  press();
  expect(bars().at(-1)?.textContent.trim()).toBe('2026-11-17: $5.00');
});

it('AIS-024: each device that reports has a line; one whose daemon sends no counts has its state and no number', async () => {
  await open();
  const [box, mac, nas] = lines('By device');
  expect(box?.querySelector('th')?.textContent).toContain("gpu-box");
  expect(box?.querySelector('th')?.textContent).toContain("Turned off in the daemon's settings");
  expect(cells(box)).toEqual([]);
  expect(mac?.querySelector('th')?.textContent).toContain('Reads sessions');
  expect(cells(mac)).toEqual(['17', '169', '19.3k', '$12.40']);
  expect(nas?.querySelector('th')?.textContent).toContain("Runs as a system unit: it cannot see a user's sessions");
});

it('the repositories stand newest use first with their last use, and a line limits the page to its repository', async () => {
  await open();
  const names = [...document.querySelectorAll('nav[aria-label="Repositories"] .line')].map((line) => [line.querySelector('.name')?.textContent, line.querySelector('.when')?.textContent]);
  expect(names).toEqual([
    ['All repositories', '30 days'],
    ['acme/anima', 'last use today'],
    ['acme/blog', 'last use 3 days ago'],
    ['No repository', 'last use today'],
  ]);
  const buttons = [...document.querySelectorAll<HTMLButtonElement>('nav[aria-label="Repositories"] button')];
  expect(buttons).toHaveLength(3);
  expect(buttons[0]?.getAttribute('aria-current')).toBe('true');
  buttons[2]?.click();
  expect(changes.at(-1)).toEqual({ ...DEFAULT_QUERY, repository: 'github.com/acme/blog' });
});

it('the list of repositories stays whole when one is chosen: it is read with no repository in the filter', async () => {
  await open({ ...DEFAULT_QUERY, repository: 'github.com/acme/blog' });
  const asked = hub.asked[0] as { filter: Record<string, string>; everything: Record<string, string> };
  expect(asked.filter['repository']).toBe('github.com/acme/blog');
  expect(asked.everything['repository']).toBeUndefined();
  const chosen = document.querySelector('nav[aria-label="Repositories"] [aria-current="true"] .name');
  expect(chosen?.textContent).toBe('acme/blog');
  expect(chosen?.getAttribute('title')).toBe('github.com/acme/blog');
});

it('the repositories that the hub left out are counted, only when there are some', async () => {
  await open();
  expect(text()).not.toContain('not shown');
  if (app !== undefined) await unmount(app);
  hub.answer = { ...ANSWER, repositoriesLeftOut: 40 };
  await open();
  expect(text()).toContain('40 more groups are not shown. The totals count them.');
});

it('AIS-020: with no daemon that sent counts the page is an empty state that says where the daemon is installed', async () => {
  hub.answer = { ...NO_USE, states: [] };
  hub.everyone = [];
  await open();
  expect(text()).toContain('No use counts yet');
  expect(text()).toContain('To install it, open a device, then Settings, then Daemon.');
  const links = [...document.querySelectorAll('a')];
  expect(links.map((link) => [link.textContent, link.getAttribute('href')])).toEqual([['Open Devices', '/devices']]);
  expect(document.querySelector('table')).toBeNull();
  expect(document.querySelector('[role="group"][aria-label="Usage filters"]')).toBeNull();
});

it('DEF-002, HMD-053: on a hub that Anima.ing hosts the empty state says where the daemon of this computer stands, and offers no link', async () => {
  hub.answer = { ...NO_USE, states: [] };
  hub.everyone = [];
  await open(DEFAULT_QUERY, 'The Anima.ing application is installing the daemon on this computer.');
  expect(text()).toContain('No use counts yet');
  expect(text()).toContain('The daemon of each device reports the use of the AI agent programs. The Anima.ing application is installing the daemon on this computer.');
  expect(text()).not.toContain('Coming soon');
  expect(text()).not.toContain('To install it');
  expect(document.querySelector('a')).toBeNull();
});

it('a device that reads but has no use in the days gives a message, with the filters and the device still there', async () => {
  hub.answer = { ...NO_USE, states: [DEVICES[1] as (typeof DEVICES)[number]] };
  await open();
  expect(text()).toContain('No use in these days');
  expect(document.querySelector('[role="group"][aria-label="Usage filters"]')).not.toBeNull();
  expect(table('By program and model')).toBeUndefined();
  expect(lines('By device')).toHaveLength(1);
});

it('while the use is read the page shows a placeholder, and a refusal shows the hub words with Retry', async () => {
  hub.fail = { code: 'UNAVAILABLE', message: 'The hub is busy.' };
  const store = new UsageStore(agentUsageApi(hub).read);
  app = mount(AgentUsageScreen, { target: document.body, props: { store, query: DEFAULT_QUERY, onQueryChange: () => undefined, onRetry: () => undefined } });
  flushSync();
  expect(document.querySelector('[role="status"][aria-busy="true"]')).not.toBeNull();
  await store.load(usageRequest(DEFAULT_QUERY, NOW));
  flushSync();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Agent usage did not load. The hub is busy.');
  expect([...document.querySelectorAll('button')].some((button) => button.textContent === 'Retry')).toBe(true);
});

it('text from a device is text: markup in a model name or a repository is not made into elements, and a long repository has its full text as a tooltip', async () => {
  const long = `repo-${'x'.repeat(200)}`;
  hub.answer = {
    ...ANSWER,
    models: [{ ...sums({ sessions: 1 }), source: 'CODEX', model: '<img src=x onerror=alert(1)>' }],
    repositories: [{ ...sums({ sessions: 1 }), repository: long }, { ...sums({ sessions: 1 }), repository: '<b>bold</b>' }],
    newest: [],
  };
  await open();
  expect(document.querySelector('tbody img')).toBeNull();
  expect(lines('By program and model')[0]?.querySelector('th')?.textContent).toContain('<img src=x onerror=alert(1)>');
  expect(document.querySelector('nav b')).toBeNull();
  expect(document.querySelector(`nav .name[title="${long}"]`)?.textContent).toBe(long);
});

it('the choices change the address: what is counted, the days, the device and the program', async () => {
  await open();
  const radio = (name: string) => document.querySelector<HTMLElement>(`[role="radio"][data-value="${name}"]`);
  radio('7')?.click();
  expect(changes.at(-1)).toEqual({ ...DEFAULT_QUERY, days: 7 });
  radio('tokens')?.click();
  expect(changes.at(-1)).toEqual({ ...DEFAULT_QUERY, count: 'tokens' });

  const choose = (label: string) => [...document.querySelectorAll<HTMLLabelElement>('label.option')].find((option) => option.textContent.includes(label))?.querySelector('input')?.click();
  control('Program')?.click();
  flushSync();
  choose('Codex');
  expect(changes.at(-1)?.source).toBe('CODEX');

  control('Device')?.click();
  flushSync();
  choose('mac-mini');
  expect(changes.at(-1)?.device).toBe('d-mac');
});

it('the device choice keeps every device when the use is filtered to one', async () => {
  hub.answer = { ...ANSWER, states: [DEVICES[1] as (typeof DEVICES)[number]] };
  hub.everyone = DEVICES;
  await open({ ...DEFAULT_QUERY, device: 'd-mac' });
  control('Device')?.click();
  flushSync();
  expect([...document.querySelectorAll('label.option')].map((option) => option.textContent.trim())).toEqual(['All devices', 'gpu-box', 'mac-mini', 'nas']);
});
