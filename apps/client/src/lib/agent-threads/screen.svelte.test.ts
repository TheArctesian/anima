// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { ThreadsStore } from './store.svelte';
import { rowsFrom, threadFrom, type Thread, type ThreadsShell } from './thread';
import ThreadsScreen from './ThreadsScreen.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const NOW = new Date(2026, 10, 20, 15, 0, 0);
const row = (id: string, title: string, minutes: number): Record<string, unknown> => ({
  id,
  program: id.startsWith('c') ? 'codex' : 'claude_code',
  title,
  folder: '/w/anima.ing',
  branch: 'dev',
  model: null,
  started: null,
  last: new Date(NOW.getTime() - minutes * 60_000).toISOString(),
});
const ROWS = [row('a1', 'Fix the sign-in test', 1), row('c2', 'Add the export', 90), row('a3', 'Old work', 4000)];

function threadOf(id: string): Thread {
  const thread = threadFrom({
    ...ROWS.find((each) => each.id === id),
    prompts: 1,
    dropped: 0,
    entries: [
      { kind: 'prompt', at: null, text: `Prompt of ${id}`, images: 0 },
      { kind: 'tool', at: null, tool: { name: 'Bash', kind: 'command', summary: 'bun test', input: '{}', patch: null, output: '1 failed', failed: true } },
      { kind: 'reply', at: null, text: 'One test fails.' },
    ],
  });
  if (thread === null) throw new Error('not a thread');
  return thread;
}

let app: ReturnType<typeof mount> | undefined;
let asked: string[];

beforeEach(() => {
  installAnimations();
  asked = [];
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function open(shell: Partial<ThreadsShell> = {}): Promise<ThreadsStore> {
  const store = new ThreadsStore({
    list: () => Promise.resolve(rowsFrom(ROWS) ?? []),
    read: (id) => {
      asked.push(id);
      return Promise.resolve(threadOf(id));
    },
    ...shell,
  });
  await store.loadList();
  app = mount(ThreadsScreen, { target: document.body, props: { store, now: () => NOW } });
  flushSync();
  return store;
}

const press = (key: string, more: KeyboardEventInit = {}): void => {
  window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...more }));
};
const titles = (): (string | null | undefined)[] => [...document.querySelectorAll('.row .words')].map((each) => each.textContent);

it('THR-013: the list shows the threads in their groups, and no thread is open at first', async () => {
  await open();
  expect([...document.querySelectorAll('section[aria-label]')].map((each) => each.getAttribute('aria-label'))).toEqual(['Changed now', 'Today', 'Earlier']);
  expect(titles()).toEqual(['Fix the sign-in test', 'Add the export', 'Old work']);
  expect(document.body.textContent).toContain('No thread is open');
  expect(asked).toEqual([]);
});

it('THR-014, THR-015: a selected thread shows its prompt, its tool call with the failure, its reply and the read-only note', async () => {
  await open();
  document.querySelector<HTMLButtonElement>('[data-thread="c2"]')?.click();
  await settle();
  expect(asked).toEqual(['c2']);
  expect(document.querySelector('[data-prompt]')?.textContent).toContain('Prompt of c2');
  expect(document.querySelector('summary .result')?.textContent).toBe('Failed');
  expect(document.querySelector('.composer')?.textContent).toContain('To continue it, use Codex');
  expect(document.querySelector('[data-thread="c2"]')?.getAttribute('aria-current')).toBe('true');
});

it('THR-016: the Down key opens the first thread, Cmd+2 the second, and E opens the tool calls', async () => {
  await open();
  press('ArrowDown');
  await settle();
  press('2', { metaKey: true });
  await settle();
  expect(asked).toEqual(['a1', 'c2']);
  expect(document.querySelector('details')?.open).toBe(false);
  press('e');
  flushSync();
  expect(document.querySelector('details')?.open).toBe(true);
});

it('THR-016: the key / goes to the search, where a letter is text and Escape leaves', async () => {
  await open();
  press('/');
  const field = document.querySelector<HTMLInputElement>('input[type="search"]');
  expect(document.activeElement).toBe(field);
  field?.dispatchEvent(new KeyboardEvent('keydown', { key: 'e', bubbles: true }));
  flushSync();
  expect(asked).toEqual([]);
  if (field) field.value = 'export';
  field?.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  expect(titles()).toEqual(['Add the export']);
  field?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  expect(document.activeElement).not.toBe(field);
});

it('THR-011: a thread that is gone shows the fixed words of the shell', async () => {
  const store = await open({ read: () => Promise.reject(new Error('This thread is not on this computer any more.')) });
  await store.open('a1');
  flushSync();
  expect(document.body.textContent).toContain('This thread is not on this computer any more.');
});
