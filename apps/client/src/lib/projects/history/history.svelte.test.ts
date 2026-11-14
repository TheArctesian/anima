// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { forgeApi } from '../forge/api';
import { NOW, fakeForgeHub, type FakeForgeHub } from '../forge/forge.fixture';
import { FIXTURE_PROJECTS } from '../projects.fixture';
import HistoryTab from './HistoryTab.svelte';

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

async function open(): Promise<void> {
  const [first] = FIXTURE_PROJECTS;
  if (first === undefined) throw new Error('no fixture project');
  const project = { ...first, repositories: first.repositories.map((each) => ({ ...each, defaultBranch: 'main' })) };
  app = mount(HistoryTab, { target: document.body, props: { project, api: forgeApi(hub), now: () => NOW } });
  for (let round = 0; round < 4; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

const texts = (selector: string) => [...document.querySelectorAll(selector)].map((each) => each.textContent.replace(/\s+/g, ' ').trim());

it('PRJ-031: commits of the default branch show by day, with their checks in words beside the mark', async () => {
  await open();
  expect(hub.calls.find((call) => call.name === 'ForgeCommits')?.variables['ref']).toBe('main');
  expect(texts('ol[aria-label="Commits"] > li')).toEqual([
    'Today',
    'Checks passed Commit a octo · 30 minutes ago aaaaaaa',
    'Yesterday',
    '1 failed Commit b octo · 1 day ago bbbbbbb',
  ]);
});

it('PRJ-031: the releases and the branches show, the default and protected branch marked', async () => {
  await open();
  expect(texts('ul[aria-label="Releases"] > li')).toEqual(['v1.0.0 First 2 days ago']);
  expect(texts('ul[aria-label="Branches"] > li')).toEqual(['main Default Protected aaaaaaa', 'feature-12 2222222']);
});
