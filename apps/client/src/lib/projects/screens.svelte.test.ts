// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { projectsApi } from './api';
import { fakeHub, type FakeHub } from './projects.fixture';
import ProjectScreen from './ProjectScreen.svelte';
import ProjectsScreen from './ProjectsScreen.svelte';
import { ProjectsStore } from './store.svelte';
import { repositoryStateWords } from './words';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: FakeHub;
let store: ProjectsStore;

beforeEach(() => {
  installAnimations();
  hub = fakeHub();
  store = new ProjectsStore(projectsApi(hub));
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

async function openList(): Promise<void> {
  await store.load();
  app = mount(ProjectsScreen, { target: document.body, props: { store } });
  flushSync();
}

async function openDetail(slug: string): Promise<void> {
  await store.open(slug);
  app = mount(ProjectScreen, { target: document.body, props: { store, slug } });
  flushSync();
}

const text = () => document.body.textContent;
const button = (label: string) => [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === label);

async function click(label: string): Promise<void> {
  button(label)?.click();
  flushSync();
  await settled();
}

it('PRO-002: the four repository states have different words', () => {
  const words = [{}, { leftOut: true }, { gone: true }, { archived: true }].map((state) => repositoryStateWords({ leftOut: false, gone: false, archived: false, ...state }));
  expect(new Set(words).size).toBe(4);
});

it('PRO-001: the list shows each project with its repository, state and resources', async () => {
  await openList();
  expect(text()).toContain('acme/acme-site');
  expect(text()).toContain('Imported');
  expect(text()).toContain('No longer on GitHub');
  expect(text()).toContain('Archived on GitHub');
  expect(text()).toContain('Cloudflare');
  expect(text()).not.toContain('acme/scratch');
});

it('PRO-002: Leave out removes the entry at once and Restore brings it back', async () => {
  await openList();
  document.querySelectorAll('li.entry')[0]?.querySelector('button')?.click();
  flushSync();
  expect(text()).not.toContain('acme/acme-site');
  await settled();
  expect(hub.sent).toContain('LeaveOutRepository:r1');
  await click('Left out');
  expect(text()).toContain('acme/acme-site');
  await click('Restore');
  expect(hub.sent).toContain('RestoreRepository:r1');
  await click('Projects');
  expect(text()).toContain('acme/acme-site');
});

it('PRO-002: a refused Leave out returns the entry and shows the hub words', async () => {
  hub.refuse = { code: 'CONFLICT', message: 'GitHub is paused for this account.' };
  await openList();
  document.querySelectorAll('li.entry')[0]?.querySelector('button')?.click();
  flushSync();
  await settled();
  expect(text()).toContain('acme/acme-site');
  expect(text()).toContain('GitHub is paused for this account.');
});

it('PRO-001: with no projects the empty state links to the Integrations section', async () => {
  store = new ProjectsStore(projectsApi(fakeHub([])));
  await openList();
  expect(text()).toContain('Connect GitHub in Settings > Integrations');
  expect(document.querySelector('a')?.getAttribute('href')).toBe('/settings#settings-integrations');
});

it('PRO-004: a proposal says Found in the repository with its sign until confirmed, then says Confirmed', async () => {
  await openDetail('acme-site');
  expect(text()).toContain('Found in the repository');
  expect(text()).toContain('wrangler.toml');
  await click('Confirm');
  expect(hub.sent).toContain('ConfirmProjectResource:x1');
  expect(text()).toContain('Confirmed');
  expect(text()).not.toContain('Found in the repository');
  expect(button('Confirm')).toBeUndefined();
});

it('PRO-004: a refused Confirm keeps the proposal and shows the hub words', async () => {
  hub.refuse = { code: 'CONFLICT', message: 'It is already confirmed.' };
  await openDetail('acme-site');
  await click('Confirm');
  expect(text()).toContain('Found in the repository');
  expect(text()).toContain('It is already confirmed.');
});

it('PRO-001: the detail page holds the name, the status and Open in GitHub in one header', async () => {
  await openDetail('acme-site');
  const header = document.querySelector('header');
  expect(header?.textContent).toContain('acme-site');
  expect(header?.textContent).toContain('Imported');
  const github = [...(header?.querySelectorAll('a') ?? [])].find((link) => link.textContent.trim() === 'Open in GitHub');
  expect(github?.getAttribute('href')).toBe('https://github.com/acme/acme-site');
  expect(text()).not.toContain('Builds');
});

it('PRO-001: an unknown slug says Project not found', async () => {
  await openDetail('missing');
  expect(text()).toContain('Project not found');
});
