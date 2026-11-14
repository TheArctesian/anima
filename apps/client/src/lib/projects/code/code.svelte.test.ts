// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { forgeApi } from '../forge/api';
import { NOW, fakeForgeHub, type FakeForgeHub } from '../forge/forge.fixture';
import { FIXTURE_PROJECTS } from '../projects.fixture';
import CodeTab from './CodeTab.svelte';

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
  app = mount(CodeTab, { target: document.body, props: { project, api: forgeApi(hub), now: () => NOW } });
  await settled();
}

async function press(name: string): Promise<void> {
  const target = [...document.querySelectorAll('button')].find((each) => each.textContent.trim() === name);
  if (target === undefined) throw new Error(`no button ${name}`);
  target.click();
  await settled();
}

const entries = () => [...document.querySelectorAll('ul[aria-label="Files"] li')].map((each) => each.textContent.replace(/\s+/g, ' ').trim());
const reads = () => hub.calls.filter((call) => call.name.startsWith('ForgeCode')).map((call) => [call.name, call.variables['ref'], call.variables['path']]);

it('PRJ-031: the root of the default branch lists folders first, and a folder and a file open with the way back', async () => {
  await open();
  expect(entries()).toEqual(['src', 'README.md 31 B', 'logo.png 2.0 KB']);
  await press('src');
  expect(entries()).toEqual(['main.ts 22 B']);
  await press('main.ts');
  expect(document.querySelector('ol[aria-label="Text of main.ts"]')?.textContent).toBe('export const answer = 42;');
  expect([...document.querySelectorAll('nav[aria-label="Path"] > *')].map((each) => each.textContent)).toEqual(['Root', '/', 'src', '/', 'main.ts']);
  await press('Root');
  expect(reads()).toEqual([
    ['ForgeCodeDirectory', 'main', ''],
    ['ForgeCodeDirectory', 'main', 'src'],
    ['ForgeCodeFile', 'main', 'src/main.ts'],
    ['ForgeCodeDirectory', 'main', ''],
  ]);
});

it('PRJ-031: a file is text with line numbers and is never rendered; a binary file is not shown', async () => {
  await open();
  await press('README.md');
  const text = document.querySelector('ol[aria-label="Text of README.md"]');
  expect([...(text?.querySelectorAll('li') ?? [])].map((each) => each.textContent)).toEqual(['# Acme', '<script>alert(1)</script>']);
  expect(document.querySelector('ol script')).toBeNull();
  await press('Root');
  await press('logo.png');
  expect(document.body.textContent).toContain('Not a text file');
});

it('PRJ-031: another branch opens at its root', async () => {
  await open();
  await press('src');
  const branch = document.querySelector<HTMLSelectElement>('select[aria-label="Branch"]');
  if (branch === null) throw new Error('no branch select');
  branch.value = 'feature-12';
  branch.dispatchEvent(new Event('change', { bubbles: true }));
  await settled();
  expect(reads().at(-1)).toEqual(['ForgeCodeDirectory', 'feature-12', '']);
});
