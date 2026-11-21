// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { account, attempt, fakeHub, NOW, type FakeHub } from '../sources.fixture';
import SourcesSection from '../SourcesSection.svelte';
import { HUB_RESTARTED, RESTARTED } from './attempt';
import type { GithubConnectApi } from './api';
import type { LinkOpening } from './browser';
import { GithubConnectStore } from './github-connect.svelte';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const TICKET = 'ONE-TIME-TICKET';
const CONSOLE = ['log', 'info', 'warn', 'error', 'debug'] as const;
const consoleSpies = CONSOLE.map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));
let app: ReturnType<typeof mount> | undefined;
let hub: FakeHub;
let opened: string[];
let closed: number;
let shown: boolean;

const opener = (): LinkOpening => ({
  open: (link) => {
    opened.push(link);
    return Promise.resolve(shown);
  },
  close: () => {
    closed += 1;
  },
});

beforeEach(() => {
  installAnimations();
  for (const spy of consoleSpies) spy.mockClear();
  localStorage.clear();
  sessionStorage.clear();
  opened = [];
  closed = 0;
  shown = true;
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

/** `connectPageUrl` of the hub; 'before': a hub of before the field. */
async function show(current: ReturnType<typeof attempt> | null = null, page: string | null = 'https://hub.example/connect/github'): Promise<void> {
  hub = fakeHub([account('a', 'Personal')]);
  hub.connectPage = page === 'before' ? undefined : page;
  hub.connect = current;
  mocks.hub = hub;
  app = mount(SourcesSection, { target: document.body, props: { now: () => NOW, opener } });
  flushSync();
  await settled();
}

function button(label: string): HTMLButtonElement | undefined {
  return [...document.querySelectorAll('button')].find((each) => each.textContent.replace(/\s+/g, ' ').trim() === label);
}

async function click(label: string): Promise<void> {
  const found = button(label);
  if (found === undefined) throw new Error(`no button ${label}`);
  found.click();
  await settled();
}

const text = () => document.body.textContent.replace(/\s+/g, ' ');
const logged = () => consoleSpies.flatMap((spy) => spy.mock.calls.flat().map(String)).join(' ');

it('PRJ-039: the GitHub row says the hub keeps the key and offers the key-file way as the second action', async () => {
  await show();
  expect(text()).toContain('Makes a GitHub App in your account; the hub keeps its key.');
  expect(button('Connect GitHub')).toBeDefined();
  await click('Use key file');
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
});

it('PRJ-041: Connect GitHub starts for the personal account, asks for the link, and hands it to the opener once', async () => {
  await show();
  await click('Connect GitHub');
  expect(hub.sent).toEqual([
    { operation: 'StartGithubConnect', variables: { input: {} } },
    { operation: 'OpenGithubConnect', variables: { id: 'attempt-1' } },
  ]);
  expect(opened).toEqual([hub.link]);
  expect(text()).toContain('Opening GitHub in your browser.');
});

it('PRJ-041: after the hand-over the one-time link is in no page text, no storage and no console call', async () => {
  await show();
  await click('Connect GitHub');
  hub.emitAttempt(attempt({ phase: 'AT_GITHUB' }));
  await settled();
  expect(document.body.innerHTML).not.toContain(TICKET);
  expect(JSON.stringify({ ...localStorage, ...sessionStorage })).not.toContain(TICKET);
  expect(logged()).not.toContain(TICKET);
  expect(opened).toHaveLength(1);
});

it('PRJ-041: a blocked tab shows a link to the attempt page, which holds no one-time value', async () => {
  shown = false;
  await show();
  await click('Connect GitHub');
  const link = document.querySelector<HTMLAnchorElement>('a[href="https://hub.example/connect/github"]');
  expect(link).not.toBeNull();
  expect(text()).toContain('The tab did not open.');
  expect(document.body.innerHTML).not.toContain(TICKET);
});

it('PRJ-039: a subscription event moves the state line with no reload', async () => {
  await show(attempt());
  expect(text()).toContain('Opening GitHub in your browser.');
  hub.emitAttempt(attempt({ phase: 'APP_CREATED', appSlug: 'anima-k3f9', ownerLogin: 'octo', installUrl: 'https://github.com/apps/anima-k3f9/installations/new' }));
  await settled();
  expect(text()).toContain('The App anima-k3f9 is made for octo. Press Install on GitHub.');
  await click('Install on GitHub');
  expect(opened).toEqual(['https://github.com/apps/anima-k3f9/installations/new']);
});

it('PRJ-041: an installUrl off github.com gives no Install action', async () => {
  await show(attempt({ phase: 'APP_CREATED', appSlug: 'x', installUrl: 'https://example.com/install' }));
  expect(button('Install on GitHub')).toBeUndefined();
});

it('PRJ-039: text from GitHub is text only', async () => {
  await show(attempt({ phase: 'APP_CREATED', appSlug: '<img src=x onerror=alert(1)>', ownerLogin: 'octo' }));
  expect(document.querySelector('img')).toBeNull();
  expect(text()).toContain('<img src=x onerror=alert(1)>');
});

it('PRJ-039: Continue in browser asks for a new link and Cancel cancels the attempt', async () => {
  await show(attempt({ phase: 'AT_GITHUB' }));
  await click('Continue in browser');
  expect(hub.sent).toEqual([{ operation: 'OpenGithubConnect', variables: { id: 'attempt-1' } }]);
  expect(opened).toEqual([hub.link]);
  await click('Cancel');
  expect(hub.sent.at(-1)).toEqual({ operation: 'CancelGithubConnect', variables: { id: 'attempt-1' } });
  expect(text()).toContain('Cancelled.');
  expect(button('Connect GitHub')).toBeDefined();
});

it('PRJ-039: a failed attempt with an App offers to delete it on GitHub, and an agent start is named', async () => {
  const settings = 'https://github.com/settings/apps/anima-k3f9/advanced';
  await show(attempt({ phase: 'FAILED', failure: 'KEY_REFUSED', startedBy: 'AGENT', appSlug: 'anima-k3f9', appSettingsUrl: settings }));
  expect(text()).toContain("An AI agent started this. GitHub refused the new App's key. Delete the App anima-k3f9 on GitHub, then start again.");
  await click('Delete App on GitHub');
  expect(opened).toEqual([settings]);
});

it('PRJ-039: when the hub holds no attempt that the client had open, the hub-restarted sentence shows', async () => {
  const reads = [attempt({ phase: 'AT_GITHUB' }), null];
  const api = {
    current: () => Promise.resolve({ ok: true as const, value: reads.shift() ?? null }),
    watch: () => () => undefined,
  } as unknown as GithubConnectApi;
  const store = new GithubConnectStore(api);
  await store.load();
  expect(store.restarted).toBe(false);
  await store.load();
  expect(store.restarted).toBe(true);
  expect(store.restarted ? RESTARTED.line : '').toBe(HUB_RESTARTED);
});

it('PRJ-039: a refused start shows the hub words and opens nothing', async () => {
  await show();
  hub.refuse = { code: 'CONFLICT', message: 'A GitHub App is made and waits for Install. Finish it or cancel it first.' };
  await click('Connect GitHub');
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('A GitHub App is made and waits for Install. Finish it or cancel it first.');
  expect(opened).toEqual([]);
  expect(closed).toBe(1);
});

for (const [what, page] of [
  ['says that it cannot start the flow in a browser', null],
  ['is of before the field, which counts as cannot', 'before'],
] as const) {
  it(`HM-T29: on a hub that ${what}, the browser flow is not offered, no link is asked for, and the key file stays`, async () => {
    await show(null, page);
    expect(text()).toContain('Coming soon: Connect GitHub in your browser for this hub.');
    expect(button('Connect GitHub')).toBeUndefined();
    expect(text()).not.toContain('must be signed in');
    expect(hub.sent).toEqual([]);
    expect(opened).toEqual([]);
    await click('Use key file');
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  });
}

it('HM-T29: a hosted hub that gives the page of its gateway offers the flow and says where the browser must be signed in', async () => {
  await show(null, 'https://hub.anima.ing/connect/github');
  expect(button('Connect GitHub')).toBeDefined();
  expect(text()).toContain('Your browser must be signed in at hub.anima.ing with your Anima.ing account.');
  expect(text()).not.toContain('Coming soon');
});

it('HM-T29: an attempt that is open on a hub that cannot start the flow can be cancelled but not continued in a browser', async () => {
  await show(attempt({ phase: 'WAITING_FOR_BROWSER' }), null);
  expect(button('Continue in browser')).toBeUndefined();
  expect(button('Install on GitHub')).toBeUndefined();
  await click('Cancel');
  expect(hub.sent.map((call) => call.operation)).toEqual(['CancelGithubConnect']);
  expect(opened).toEqual([]);
});
