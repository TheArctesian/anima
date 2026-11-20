// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { projectsApi } from '../api';
import { fakeHub, type FakeHub } from '../projects.fixture';
import ProjectsScreen from '../ProjectsScreen.svelte';
import { ProjectsStore } from '../store.svelte';
import { storedClusters } from './clusters';
import { ProjectGroups } from './groups.svelte';
import { storedShowBy } from './view';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: FakeHub;
let store: ProjectsStore;
let ids = 0;
const groups = () => new ProjectGroups(localStorage, () => `g${String((ids += 1))}`);

beforeEach(() => {
  installAnimations();
  localStorage.clear();
  ids = 0;
  hub = fakeHub();
  store = new ProjectsStore(projectsApi(hub));
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function openList(): Promise<void> {
  await store.load();
  app = mount(ProjectsScreen, { target: document.body, props: { store, projectGroups: groups() } });
  flushSync();
}

const text = () => document.body.textContent;
/** Each group head as its name and its count. */
const heads = () => [...document.querySelectorAll('button[aria-expanded]')].map((each) => each.textContent.replace(/\s+/g, '').replace(/(\d+)$/, ' $1'));

function click(label: string): void {
  [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === label)?.click();
  flushSync();
}

function check(name: string): void {
  document.querySelector<HTMLInputElement>(`input[aria-label="Select ${name}"]`)?.click();
  flushSync();
}

function nameGroup(name: string, save: string): void {
  const field = document.querySelector<HTMLInputElement>('form#group-name input');
  if (field === null) throw new Error('no name field');
  field.value = name;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  // The dialog's button is the last with this label: a group head also has a Rename.
  [...document.querySelectorAll('button')].findLast((each) => each.textContent.trim() === save)?.click();
  flushSync();
}

it('PRO-060: New group puts the selected cards under a name, kept on this computer, and sends nothing to the hub', async () => {
  await openList();
  click('Select');
  check('acme-site');
  check('notes');
  expect(text()).toContain('2 selected');
  click('New group');
  nameGroup('Work', 'Make group');
  click('Group');
  expect(heads()).toEqual(['Work 2', 'Ungrouped 1']);
  expect(storedClusters(localStorage).groups.map((group) => group.name)).toEqual(['Work']);
  expect(hub.sent).toEqual([]);
});

it('PRO-060: Select all takes each card the list shows, and Add to group moves a card from its other group', async () => {
  await openList();
  click('Select');
  check('notes');
  click('New group');
  nameGroup('Home', 'Make group');
  click('Select all');
  expect(text()).toContain('3 selected');
  click('New group');
  nameGroup('home', 'Make group');
  expect(text()).toContain('A group already has this name.');
  nameGroup('Work', 'Make group');
  check('notes');
  const add = document.querySelector<HTMLSelectElement>('[aria-label="Selected projects"] select');
  if (add === null) throw new Error('no Add to group');
  add.value = 'g1';
  add.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
  click('Group');
  expect(heads()).toEqual(['Home 1', 'Work 2']);
});

it('PRO-061: the view by group and a closed head are kept on this computer', async () => {
  await openList();
  click('Select');
  check('notes');
  click('New group');
  nameGroup('Home', 'Make group');
  click('Group');
  expect(storedShowBy(localStorage)).toBe('group');
  document.querySelector<HTMLButtonElement>('button[aria-controls="group-none"]')?.click();
  flushSync();
  expect(text()).not.toContain('acme/acme-site');
  await unmount(app);
  app = mount(ProjectsScreen, { target: document.body, props: { store, projectGroups: groups() } });
  flushSync();
  expect(heads()).toEqual(['Home 1', 'Ungrouped 2']);
  expect(text()).not.toContain('acme/acme-site');
  expect(text()).toContain('acme/notes');
});

it('PRO-062 and PRO-063: rename, remove from a group and delete a group change no project', async () => {
  await openList();
  click('Select');
  check('acme-site');
  check('notes');
  click('New group');
  nameGroup('Work', 'Make group');
  check('notes');
  click('Remove from group');
  click('Done');
  click('Group');
  expect(heads()).toEqual(['Work 1', 'Ungrouped 2']);
  click('Rename');
  nameGroup('Sites', 'Rename');
  expect(heads()).toEqual(['Sites 1', 'Ungrouped 2']);
  click('Delete');
  click('Delete group');
  // The dialog and the head leave with their motion before the section goes.
  for (let turn = 0; turn < 3; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
  expect(heads()).toEqual(['Ungrouped 3']);
  expect(store.main).toHaveLength(3);
  expect(hub.sent).toEqual([]);
});

it('PRO-059: a storage that refuses or holds a broken value gives no group, and the screen still works', () => {
  const refusing = {
    getItem: () => {
      throw new Error('denied');
    },
    setItem: () => {
      throw new Error('denied');
    },
  } as unknown as Storage;
  const kept = new ProjectGroups(refusing, () => 'g1');
  expect(kept.create('Work', ['p1'])).toBeNull();
  expect(kept.of('p1')?.name).toBe('Work');
  localStorage.setItem('anima.projects.groups', '{"groups": 3');
  expect(storedClusters(localStorage)).toEqual({ groups: [], of: {} });
});
