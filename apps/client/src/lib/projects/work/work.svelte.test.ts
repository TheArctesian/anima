// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { forgeApi } from '../forge/api';
import { NOW, fakeForgeHub, pull, run, type FakeForgeHub } from '../forge/forge.fixture';
import { FIXTURE_PROJECTS } from '../projects.fixture';
import WorkTab from './WorkTab.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: FakeForgeHub;

beforeEach(() => {
  installAnimations();
  hub = fakeForgeHub();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settled(): Promise<void> {
  for (let round = 0; round < 4; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

async function open(): Promise<void> {
  const [first] = FIXTURE_PROJECTS;
  if (first === undefined) throw new Error('no fixture project');
  const project = { ...first, repositories: first.repositories.map((each) => ({ ...each, defaultBranch: 'main' })) };
  app = mount(WorkTab, { target: document.body, props: { project, api: forgeApi(hub), now: () => NOW } });
  await settled();
}

const named = (name: string) => hub.calls.filter((call) => call.name === name);
const button = (label: string) =>
  [...document.querySelectorAll('button')].find((each) => each.getAttribute('aria-label') === label || each.textContent.trim() === label);
const dialog = () => document.querySelector('[role="dialog"]');

/** The dialog's yes: the last button of its footer. */
async function yes(): Promise<void> {
  ([...(dialog()?.querySelectorAll('button') ?? [])].at(-1) as HTMLButtonElement).click();
  await settled();
}

async function press(label: string): Promise<void> {
  const target = button(label);
  if (target === undefined) throw new Error(`no button ${label}`);
  target.click();
  await settled();
}

it('PRJ-032: a repository the hub cannot read says why, and no list is asked for', async () => {
  hub.data.access = 'NOT_CONNECTED';
  await open();
  expect(document.body.textContent).toContain('Not connected');
  expect(hub.calls.map((call) => call.name)).toEqual(['ProjectAccess']);
});

it('PRJ-031: runs, pull requests and issues show, each with when it was read', async () => {
  await open();
  const lists = [...document.querySelectorAll('ul[aria-label]')].map((each) => each.getAttribute('aria-label'));
  expect(lists).toEqual(['Workflow runs', 'Pull requests', 'Issues']);
  expect(document.querySelectorAll('ul[aria-label="Pull requests"] > li')).toHaveLength(3);
  expect(document.body.textContent).toContain('Read just now');
  expect(hub.calls.map((call) => call.name)).toEqual(['ProjectAccess', 'ForgeWorkflowRuns', 'ForgePullRequests', 'ForgeIssues']);
});

it('PRJ-032: Refresh asks GitHub again, and a failed refresh keeps the rows and says so', async () => {
  await open();
  hub.failing.add('ForgeIssues');
  const refresh = [...document.querySelectorAll('section')].find((each) => each.textContent.includes('Issues'))?.querySelector('header button');
  (refresh as HTMLElement).click();
  await settled();
  expect(named('ForgeIssues').at(-1)?.variables['fresh']).toBe(true);
  expect(document.querySelectorAll('ul[aria-label="Issues"] > li')).toHaveLength(2);
  expect(document.body.textContent).toContain('Refresh failed');
});

it('PRJ-033, PRO-013: only clean pull requests from this repository can be picked, and they merge at the commit shown', async () => {
  hub.data.pulls = [pull(13), ...hub.data.pulls];
  await open();
  const box = (number: number) => document.querySelector<HTMLInputElement>(`input[aria-label="Select #${String(number)} to merge"]`);
  expect(box(11)?.disabled).toBe(true);
  expect(box(10)?.disabled).toBe(true);
  box(13)?.click();
  box(12)?.click();
  await settled();
  await press('Merge 2');
  expect(dialog()?.textContent).toContain('2222222');
  expect(named('ForgeMergePullRequests')).toEqual([]);
  await yes();
  expect(named('ForgeMergePullRequests')[0]?.variables).toMatchObject({
    pullRequests: [
      { repository: 'r1', number: 13, expectedHeadSha: '3'.repeat(40) },
      { repository: 'r1', number: 12, expectedHeadSha: '2'.repeat(40) },
    ],
  });
  expect(document.querySelector('[aria-label="Merge results"]')?.textContent).toContain('Merged.');
});

it('PRO-013: a pull request from a fork merges alone, only after its repository is typed', async () => {
  await open();
  await press('Merge #10');
  const merge = [...(dialog()?.querySelectorAll('button') ?? [])].at(-1) as HTMLButtonElement;
  expect(merge.getAttribute('aria-disabled')).toBe('true');
  merge.click();
  await settled();
  expect(named('ForgeMergePullRequest')).toEqual([]);
  const typed = dialog()?.querySelector('input') as HTMLInputElement;
  typed.value = 'stranger/acme-site';
  typed.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  merge.click();
  await settled();
  expect(named('ForgeMergePullRequest')[0]?.variables['input']).toMatchObject({ number: 10, forkConfirmed: 'stranger/acme-site' });
});

it('PRJ-034: re-running failed jobs names the commit, sends nothing before the yes, and sends the head it showed', async () => {
  await open();
  await press('Re-run failed jobs of CI #899');
  expect(dialog()?.textContent).toContain(`Commit ${'a'.repeat(40)}`);
  expect(named('ForgeRerunFailedWorkflowJobs')).toEqual([]);
  await yes();
  expect(named('ForgeRerunFailedWorkflowJobs')[0]?.variables).toEqual({ repository: 'r1', runId: '899', expectedHeadSha: 'a'.repeat(40) });
  expect(dialog()).toBeNull();
});

it('PRJ-034: while a run is going the runs are read again every 30 seconds, and the reading stops when none is', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  try {
    await open();
    vi.advanceTimersByTime(30_000);
    await settled();
    expect(named('ForgeWorkflowRuns').map((call) => call.variables['fresh'])).toEqual([false, true]);
    hub.data.runs = [run('900')];
    vi.advanceTimersByTime(30_000);
    await settled();
    vi.advanceTimersByTime(60_000);
    await settled();
    expect(named('ForgeWorkflowRuns')).toHaveLength(3);
  } finally {
    vi.useRealTimers();
  }
});

it('PRJ-028: a new issue is made on GitHub and opens in the list', async () => {
  await open();
  await press('New issue');
  const title = dialog()?.querySelector('input') as HTMLInputElement;
  title.value = 'Logs jump on Safari';
  title.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  await yes();
  expect(named('ForgeCreateIssue')[0]?.variables).toEqual({ input: { repository: 'r1', title: 'Logs jump on Safari', body: null } });
  expect(document.querySelector('ul[aria-label="Issues"] > li')?.textContent).toContain('Logs jump on Safari');
});

it('PRJ-031: an issue and its comments are plain text, a reply is sent, and closing takes it out of Open', async () => {
  await open();
  await press('Issue 5');
  const opened = document.querySelector('ul[aria-label="Issues"] > li');
  expect(opened?.textContent).toContain('<b>Logs</b> do not scroll');
  expect(opened?.querySelector('b')).toBeNull();
  expect(opened?.textContent).toContain('Seen on **Safari**');
  const reply = opened?.querySelector('textarea') as HTMLTextAreaElement;
  reply.value = 'Fixed in 1.2';
  reply.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  await press('Comment');
  expect(named('ForgeAddIssueComment')[0]?.variables).toEqual({ repository: 'r1', number: 5, body: 'Fixed in 1.2' });
  expect(document.querySelectorAll('ol[aria-label="Comments"] > li')).toHaveLength(2);
  await press('Close as done');
  expect(named('ForgeSetIssueOpen')[0]?.variables).toEqual({ repository: 'r1', number: 5, open: false, reason: 'COMPLETED' });
  expect([...document.querySelectorAll('ul[aria-label="Issues"] > li')].map((each) => each.textContent)).not.toContain('Issue 5');
});
