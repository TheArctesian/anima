// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { agentUsageApi } from './api';
import { ANSWER, DEVICES, fakeUsageHub, row, type FakeUsageHub } from './agent-usage.fixture';
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

async function open(query: UsageQuery = DEFAULT_QUERY, hosted = false): Promise<UsageStore> {
  const store = new UsageStore(agentUsageApi(hub).read);
  const reading = store.load(usageRequest(query, NOW));
  app = mount(AgentUsageScreen, {
    target: document.body,
    props: { store, query, hosted, onQueryChange: (next: UsageQuery) => changes.push(next), onRetry: () => void store.load(usageRequest(query, NOW)) },
  });
  flushSync();
  await reading;
  flushSync();
  return store;
}

const text = () => document.body.textContent;
const control = (label: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button.control')].find((button) => (button.getAttribute('aria-label') ?? button.textContent.trim()).startsWith(label));
const cells = (rowIndex: number) => [...document.querySelectorAll('tbody tr')[rowIndex]?.querySelectorAll('td') ?? []].map((cell) => cell.textContent);

it('AIS-020: the summary line gives the sessions, all tokens and the estimate, as text', async () => {
  await open();
  expect(document.querySelector('.summary')?.textContent).toBe('17 sessions, 1.4B tokens, estimated cost $12.40');
});

it('AIS-020: the key columns are the group the operator chose, then the counts', async () => {
  await open({ ...DEFAULT_QUERY, group: ['DEVICE', 'SOURCE'] });
  const headers = [...document.querySelectorAll('thead th')].map((header) => header.textContent);
  expect(headers).toEqual(['Device', 'Program', 'Sessions', 'Prompts', 'Tool calls', 'Input', 'Output', 'Cache read', 'Cache write', 'Est. cost (USD)']);
  expect(hub.asked[0]?.['by']).toEqual(['DEVICE', 'SOURCE']);
});

it('AIS-020: a different group gives different key columns', async () => {
  hub.answer = { ...ANSWER, rows: [row({ day: '2026-11-17', model: 'claude-opus-4', repository: null, sessions: 2, estimatedCostUsd: 1 })] };
  await open({ ...DEFAULT_QUERY, group: ['DAY', 'MODEL', 'REPOSITORY'] });
  expect([...document.querySelectorAll('thead th')].slice(0, 3).map((header) => header.textContent)).toEqual(['Day', 'Model', 'Repository']);
  expect(cells(0).slice(0, 3)).toEqual(['2026-11-17', 'claude-opus-4', 'No repository']);
});

it('AIS-021: tokens read in the units of the command, the estimate in dollars, and a model with no price reads No price', async () => {
  await open();
  expect(cells(0)).toEqual(['mac-mini', 'Claude Code', '12', '140', '1210', '18.3k', '420.0k', '2.2M', '61.0k', '12.40']);
  expect(cells(1).at(-1)).toBe('<0.01');
  expect(cells(2)).toContain('1.4B');
  expect(cells(2).at(-1)).toBe('No price');
});

it('AIS-021: under the table the estimate says it is not the bill of a subscription', async () => {
  await open();
  expect(text()).toContain('The cost is an estimate from the public list prices of 2026-11-01. It is not the bill of a subscription.');
});

it('the tokens with no price are named, and the rows left out are counted, only when there are some', async () => {
  hub.answer = { ...ANSWER, rowsLeftOut: 40 };
  await open();
  expect(text()).toContain('1.4B tokens have no price');
  expect(text()).toContain('40 rows are not shown.');
});

it('with every model priced and no row left out, neither note is there', async () => {
  hub.answer = { ...ANSWER, total: { ...ANSWER.total, unpricedTokens: 0 } };
  await open();
  expect(text()).not.toContain('have no price');
  expect(text()).not.toContain('not shown');
});

it('AIS-024: a device whose daemon is a system unit gives the reason, and a device turned off says it is off in the settings', async () => {
  await open();
  expect(text()).toContain('nas: This daemon runs as a system unit and cannot see the sessions of a user.');
  expect(text()).toContain('gpu-box: The use counts are turned off in the settings of the daemon on this device.');
  expect(text()).not.toContain('mac-mini: ');
});

it('AIS-020: with no daemon that sent counts the page is an empty state that says where the daemon is installed', async () => {
  hub.answer = { ...ANSWER, rows: [], devices: [] };
  hub.everyone = [];
  await open();
  expect(text()).toContain('No use counts yet');
  expect(text()).toContain('To install it, open a device, then Settings, then Daemon.');
  const links = [...document.querySelectorAll('a')];
  expect(links.map((link) => [link.textContent, link.getAttribute('href')])).toEqual([['Open Devices', '/devices']]);
  expect(document.querySelector('table')).toBeNull();
});

it('DEF-002: on a hub that Anima.ing hosts the empty state says that a daemon cannot connect yet, and offers no link', async () => {
  hub.answer = { ...ANSWER, rows: [], devices: [] };
  hub.everyone = [];
  await open(DEFAULT_QUERY, true);
  expect(text()).toContain('No use counts yet');
  expect(text()).toContain('A daemon cannot connect to a hub that Anima.ing hosts yet');
  expect(text()).not.toContain('To install it');
  expect(document.querySelector('a')).toBeNull();
});

it('a device that reads but has no use in the days gives an empty table message, with the filters still there', async () => {
  hub.answer = { ...ANSWER, rows: [], total: row({}), devices: [DEVICES[1] as (typeof DEVICES)[number]] };
  await open();
  expect(text()).toContain('No use in these days');
  expect(document.querySelector('[role="group"][aria-label="Usage filters"]')).not.toBeNull();
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

it('text from a device is text: markup in a model name is not made into elements, and a long value has its full text as a tooltip', async () => {
  const long = `repo-${'x'.repeat(200)}`;
  hub.answer = { ...ANSWER, rows: [row({ model: '<img src=x onerror=alert(1)>', repository: long, sessions: 1 })] };
  await open({ ...DEFAULT_QUERY, group: ['MODEL', 'REPOSITORY'] });
  expect(document.querySelector('tbody img')).toBeNull();
  expect(cells(0)[0]).toBe('<img src=x onerror=alert(1)>');
  expect(document.querySelectorAll('tbody td')[1]?.getAttribute('title')).toBe(long);
});

it('the choices change the address: days, group, device and program', async () => {
  await open();
  const radio = (name: string) => document.querySelector<HTMLElement>(`[role="radio"][data-value="${name}"]`);
  radio('7')?.click();
  expect(changes.at(-1)).toEqual({ ...DEFAULT_QUERY, days: 7 });

  control('Group by')?.click();
  flushSync();
  const choose = (label: string) => [...document.querySelectorAll<HTMLLabelElement>('label.option')].find((option) => option.textContent.includes(label))?.querySelector('input')?.click();
  choose('Model');
  expect(changes.at(-1)?.group).toEqual(['DEVICE', 'SOURCE', 'MODEL']);

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
  hub.answer = { ...ANSWER, devices: [DEVICES[1] as (typeof DEVICES)[number]] };
  hub.everyone = DEVICES;
  await open({ ...DEFAULT_QUERY, device: 'd-mac' });
  control('Device')?.click();
  flushSync();
  expect([...document.querySelectorAll('label.option')].map((option) => option.textContent.trim())).toEqual(['All devices', 'gpu-box', 'mac-mini', 'nas']);
});
