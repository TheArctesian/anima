// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { account, fakeHub, NOW, run, type FakeHub } from './sources.fixture';
import SourcesSection from './SourcesSection.svelte';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const KEY = '-----BEGIN RSA PRIVATE KEY-----\nQUJDREVGR0hJSktM\n-----END RSA PRIVATE KEY-----';
const KEY_BODY = 'QUJDREVGR0hJSktM';
const CONSOLE = ['log', 'info', 'warn', 'error', 'debug'] as const;
const consoleSpies = CONSOLE.map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));
let app: ReturnType<typeof mount> | undefined;
let hub: FakeHub;

beforeEach(() => {
  installAnimations();
  for (const spy of consoleSpies) spy.mockClear();
  localStorage.clear();
  sessionStorage.clear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function open(...accounts: ReturnType<typeof account>[]): Promise<void> {
  hub = fakeHub(accounts);
  mocks.hub = hub;
  app = mount(SourcesSection, { target: document.body, props: { now: () => NOW } });
  flushSync();
  await settled();
}

function button(label: string): HTMLButtonElement {
  // The disclosure's button also holds its chevron glyph, so a disclosure is found by the start of its text.
  const same = (each: HTMLButtonElement) => each.textContent.replace(/\s+/g, ' ').trim() === label || (each.hasAttribute('aria-expanded') && each.textContent.includes(label));
  const found = [...document.querySelectorAll('button')].find(same);
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

async function click(label: string): Promise<void> {
  button(label).click();
  await settled();
}

function type(selector: string, value: string): void {
  const input = document.querySelector<HTMLInputElement>(selector);
  if (input === null) throw new Error(`no field ${selector}`);
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

async function fillAndSend(privateKey: string): Promise<void> {
  await click('Connect GitHub');
  type('input[id$="-label"]', 'Personal');
  type('input[id$="-appId"]', '12345');
  type('input[id$="-installationId"]', '678');
  type('input[id$="-privateKey"]', privateKey);
  await click('Connect');
}

const text = () => document.body.textContent.replace(/\s+/g, ' ');
const keyField = () => document.querySelector<HTMLInputElement>('input[id$="-privateKey"]');

it('SRC-003, ITG-002: a row shows the label, the source, the health, the last sync and the last error', async () => {
  await open(account('a', 'Personal', { lastError: 'GitHub did not answer.', health: 'STALLED' }));
  expect(text()).toContain('Personal');
  expect(text()).toContain('GitHub. Stalled.');
  expect(text()).toContain('Synced 4 min ago');
  expect(text()).toContain('Last error: GitHub did not answer.');
});

it('SRC-009: every health state shows its own words', async () => {
  await open(
    account('1', 'one', { health: 'OK' }),
    account('2', 'two', { health: 'STARTING' }),
    account('3', 'three', { health: 'STALLED' }),
    account('4', 'four', { health: 'NEVER_SYNCED', lastSyncAt: null }),
    account('5', 'five', { health: 'PAUSED', pausedBy: 'OPERATOR' }),
    account('6', 'six', { health: 'NEEDS_RECONNECT' }),
  );
  for (const words of ['OK. It reads on schedule.', 'Starting. The first sync is running.', 'Stalled. Syncs keep failing', 'Never synced. No sync has finished yet.', 'Paused. You paused it.', 'Needs reconnect. The source refused the key.']) {
    expect(text()).toContain(words);
  }
});

it('SRC-005: the recent syncs list shows the trigger, the times and the four counts', async () => {
  await open(account('a', 'Personal', { runs: [run('r1', { trigger: 'MANUAL', inserted: 7, changed: 2, unchanged: 40, removed: 1 })] }));
  await click('Recent syncs');
  expect(text()).toContain('By hand. Started 10 min ago. Ended 10 min ago. Done.');
  expect(text()).toContain('Inserted 7');
  expect(text()).toContain('Changed 2');
  expect(text()).toContain('Unchanged 40');
  expect(text()).toContain('Removed 1');
});

it('SRC-005: a change the hub sends updates its row with no reload', async () => {
  await open(account('a', 'Personal'), account('b', 'Work'));
  hub.emit(account('a', 'Personal', { health: 'NEEDS_RECONNECT', lastError: 'GitHub refused the key.' }));
  await settled();
  expect(text()).toContain('Needs reconnect. The source refused the key.');
  expect(text()).toContain('Last error: GitHub refused the key.');
  expect(text()).toContain('OK. It reads on schedule.');
  expect(hub.sent).toEqual([]);
});

it('SRC-003: Sync now and Pause send the account and show what the hub answers', async () => {
  await open(account('a', 'Personal'));
  await click('Sync now');
  await click('Pause');
  expect(hub.sent).toEqual([
    { operation: 'SyncSourceAccount', variables: { id: 'a' } },
    { operation: 'SetSourceAccountPaused', variables: { id: 'a', paused: true } },
  ]);
  expect(text()).toContain('Paused. You paused it.');
  await click('Resume');
  expect(hub.sent.at(-1)).toEqual({ operation: 'SetSourceAccountPaused', variables: { id: 'a', paused: false } });
});

it('SRC-009: a refused action shows the hub words on that row and on no other', async () => {
  await open(account('a', 'Personal'), account('b', 'Work'));
  hub.refuse = { code: 'CONFLICT', message: 'A sync of this account is already running.' };
  const rowOf = (label: string) => [...document.querySelectorAll('li')].find((each) => each.textContent.includes(label) && each.querySelector('button') !== null);
  document.querySelectorAll<HTMLButtonElement>('button').forEach((each) => {
    if (each.textContent.trim() === 'Sync now' && rowOf('Personal')?.contains(each) === true) each.click();
  });
  await settled();
  const alerts = [...document.querySelectorAll('[role="alert"]')].map((each) => each.textContent.trim()).filter((words) => words !== '');
  expect(alerts).toEqual(['A sync of this account is already running.']);
  expect(rowOf('Personal')?.textContent).toContain('already running');
  expect(rowOf('Work')?.textContent).not.toContain('already running');
});

it('SRC-003: Disconnect names the account, asks keep or delete, and sends the choice', async () => {
  await open(account('a', 'Personal'), account('b', 'Work'));
  await click('Disconnect');
  expect(text()).toContain('Disconnect Personal');
  expect(text()).toContain('Keep the records');
  expect(text()).toContain('Delete the records');
  expect(hub.sent).toEqual([]);
  await click('Disconnect Personal');
  expect(hub.sent).toEqual([{ operation: 'RemoveSourceAccount', variables: { id: 'a', deleteRecords: false } }]);
  expect(text()).not.toContain('Personal');
  expect(text()).toContain('Work');
});

it('SRC-003: choosing Delete the records sends deleteRecords true', async () => {
  await open(account('a', 'Personal'));
  await click('Disconnect');
  const choice = [...document.querySelectorAll<HTMLInputElement>('input[type="radio"]')].find((each) => each.value === 'delete');
  choice?.click();
  await settled();
  await click('Disconnect Personal');
  expect(hub.sent).toEqual([{ operation: 'RemoveSourceAccount', variables: { id: 'a', deleteRecords: true } }]);
});

it('PRJ-018: Connect GitHub says the operator makes the App, and sends the label, both IDs and the key', async () => {
  await open();
  await click('Connect GitHub');
  expect(text()).toContain('You make the GitHub App in your own account');
  await fillAndSend(KEY);
  const sent = hub.sent[0];
  expect(sent?.operation).toBe('ConnectGithubApp');
  expect(sent?.variables['input']).toEqual({ label: 'Personal', appId: '12345', installationId: '678', privateKey: KEY });
  expect(text()).toContain('Personal');
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('PRJ-018: after a send the key is nowhere: not in the page, a reopened dialog is empty, no storage, no log', async () => {
  await open();
  await fillAndSend(KEY);
  await click('Connect GitHub');
  expect(keyField()?.value).toBe('');
  expect(document.body.innerHTML).not.toContain(KEY_BODY);
  expect(JSON.stringify({ ...localStorage })).not.toContain(KEY_BODY);
  expect(JSON.stringify({ ...sessionStorage })).not.toContain(KEY_BODY);
  expect(consoleSpies.flatMap((spy) => spy.mock.calls.flat().map(String)).join(' ')).not.toContain(KEY_BODY);
});

it('PRJ-018: after a refused connect the key field is empty, the other fields stay, and the hub words show', async () => {
  await open();
  await click('Connect GitHub');
  hub.refuse = { code: 'CONFLICT', message: 'GitHub refused this App ID.' };
  type('input[id$="-label"]', 'Personal');
  type('input[id$="-appId"]', '12345');
  type('input[id$="-installationId"]', '678');
  type('input[id$="-privateKey"]', KEY);
  await click('Connect');
  expect(keyField()?.value).toBe('');
  expect(document.querySelector<HTMLInputElement>('input[id$="-appId"]')?.value).toBe('12345');
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('GitHub refused this App ID.');
  expect(document.body.innerHTML).not.toContain(KEY_BODY);
  await click('Cancel');
  await click('Connect GitHub');
  expect(keyField()?.value).toBe('');
  expect(document.querySelector<HTMLInputElement>('input[id$="-appId"]')?.value).toBe('');
});

it('PRJ-018: the key field hides what is typed, is not named for the browser, and a form with no key sends nothing', async () => {
  await open();
  await click('Connect GitHub');
  expect(keyField()?.type).toBe('password');
  expect(keyField()?.hasAttribute('name')).toBe(false);
  await click('Connect');
  expect(hub.sent).toEqual([]);
  expect(text()).toContain('Paste the private key or choose its .pem file.');
});
