// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { projectsApi } from '../api';
import { FIXTURE_PROJECTS, fakeHub, type FakeHub } from '../projects.fixture';
import ProjectsScreen from '../ProjectsScreen.svelte';
import { ProjectsStore } from '../store.svelte';
import { sectionsByGroup, storedShowBy, storeView } from './view';

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
  localStorage.clear();
  hub = fakeHub(FIXTURE_PROJECTS, [{ id: 'home', name: 'Home', projectCount: 0 }]);
  store = new ProjectsStore(projectsApi(hub));
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settled(): Promise<void> {
  for (let turn = 0; turn < 3; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function openList(): Promise<void> {
  await store.load();
  app = mount(ProjectsScreen, { target: document.body, props: { store } });
  flushSync();
}

const text = () => document.body.textContent;
const button = (label: string) => [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === label);
/** Each group head as its name and its count. */
const heads = () => [...document.querySelectorAll('button[aria-expanded]')].map((each) => each.textContent.replace(/\s+/g, '').replace(/(\d+)$/, ' $1'));

async function click(label: string): Promise<void> {
  button(label)?.click();
  flushSync();
  await settled();
}

function check(name: string): void {
  document.querySelector<HTMLInputElement>(`input[aria-label="Select ${name}"]`)?.click();
  flushSync();
}

async function nameGroup(name: string, save: string): Promise<void> {
  const field = document.querySelector<HTMLInputElement>('form#group-name input');
  if (field === null) throw new Error('no name field');
  field.value = name;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  // The dialog's button is the last with this label: a group head also has a Rename.
  [...document.querySelectorAll('button')].findLast((each) => each.textContent.trim() === save)?.click();
  flushSync();
  await settled();
}

it('PRO-060: New group makes the group with the selected projects, and the group view shows it with its count', async () => {
  await openList();
  await click('Select');
  check('acme-site');
  check('notes');
  expect(text()).toContain('2 selected');
  await click('New group');
  await nameGroup('Work', 'Make group');
  expect(hub.sent).toContain('CreateProjectGroup:');
  await click('Group');
  expect(heads()).toEqual(['Home 0', 'Work 2', 'Ungrouped 1']);
});

it('PRO-060: Select all takes each project that the present filter shows, and Add to group moves them', async () => {
  await openList();
  await store.setBrowse({ ungrouped: true });
  flushSync();
  await click('Select');
  await click('Select all');
  expect(text()).toContain('3 selected');
  const add = document.querySelector<HTMLSelectElement>('[aria-label="Selected projects"] select');
  if (add === null) throw new Error('no Add to group');
  add.value = 'home';
  add.dispatchEvent(new Event('change', { bubbles: true }));
  await settled();
  expect(hub.sent).toContain('SetProjectsGroup:');
  expect(store.main.every((project) => project.group?.name === 'Home')).toBe(true);
});

it('PRO-061: the group view is kept on the computer and a head closes its group', async () => {
  await openList();
  await click('Group');
  expect(storedShowBy(localStorage)).toBe('group');
  document.querySelector<HTMLButtonElement>('button[aria-controls="group-none"]')?.click();
  flushSync();
  await settled();
  expect(text()).not.toContain('acme/acme-site');
  await unmount(app);
  app = mount(ProjectsScreen, { target: document.body, props: { store } });
  flushSync();
  expect(heads()).toEqual(['Home 0', 'Ungrouped 3']);
  expect(text()).not.toContain('acme/acme-site');
});

it('PRO-062 and PRO-063: deleting a group and removing a project keep every project and leave none out', async () => {
  await openList();
  await click('Select');
  check('acme-site');
  check('notes');
  await click('New group');
  await nameGroup('Work', 'Make group');
  check('notes');
  await click('Remove from group');
  await click('Done');
  await click('Group');
  expect(heads()).toEqual(['Home 0', 'Work 1', 'Ungrouped 2']);
  const work = document.querySelector('section[aria-label="Work"]');
  [...(work?.querySelectorAll('button') ?? [])].find((each) => each.textContent.trim() === 'Delete')?.click();
  flushSync();
  await click('Delete group');
  expect(heads()).toEqual(['Home 0', 'Ungrouped 3']);
  expect(store.main).toHaveLength(3);
  expect(hub.sent.filter((each) => each.startsWith('LeaveOut'))).toEqual([]);
});

it('PRO-062: Rename shows the hub words when it refuses', async () => {
  await openList();
  await click('Group');
  hub.refuse = { code: 'CONFLICT', message: 'A project group already has this name.' };
  await click('Rename');
  await nameGroup('Work', 'Rename');
  expect(text()).toContain('A project group already has this name.');
});

it('PRO-061: the groups come in name order, then Ungrouped; a storage that refuses keeps the status view', () => {
  const [one, two] = FIXTURE_PROJECTS;
  const projects = [
    { ...one, group: { id: 'b', name: 'beta' } },
    { ...two, group: null },
  ].filter((project) => project.id !== undefined) as typeof FIXTURE_PROJECTS;
  const groups = [
    { id: 'b', name: 'beta', projectCount: 1 },
    { id: 'a', name: 'Alpha', projectCount: 0 },
  ];
  expect(sectionsByGroup(projects, groups, true).map((each) => each.title)).toEqual(['Alpha', 'beta', 'Ungrouped']);
  expect(sectionsByGroup(projects, groups, false).map((each) => each.title)).toEqual(['beta', 'Ungrouped']);
  const refusing = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } } as unknown as Storage;
  expect(() => storeView(refusing, 'group', [])).not.toThrow();
  expect(storedShowBy(refusing)).toBe('status');
});
