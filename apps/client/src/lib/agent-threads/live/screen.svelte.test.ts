// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { ThreadsStore } from '../store.svelte';
import ThreadsScreen from '../ThreadsScreen.svelte';
import { askFrame, madeShell, saidFrame, stateFrame, THREAD } from './frames.fixture';
import { LiveThreads } from './live.svelte';

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const NOW = new Date('2026-11-20T10:05:00Z');
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
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

async function openLive(): Promise<ReturnType<typeof madeShell>> {
  const shell = madeShell();
  const live = new LiveThreads(shell, () => ({ front: true, chosen: null }), () => NOW);
  await live.begin();
  shell.emit(stateFrame(1, 'working'));
  shell.emit(saidFrame(2, { kind: 'message', text: 'Look: <img src=x onerror="window.reached = true"> **bold**', parent: null }));
  shell.emit(askFrame(3, '0190a000-0000-7000-8000-0000000000aa', '<script>window.reached = true</script>'));
  const store = new ThreadsStore({ list: () => Promise.resolve([]), read: () => Promise.reject(new Error('no file')) }, null, () => NOW, live);
  await store.loadList();
  app = mount(ThreadsScreen, { target: document.body, props: { store, now: () => NOW } });
  flushSync();
  await store.open(THREAD);
  await settle();
  return shell;
}

it('THR-032: the text of a driven thread and of its request is drawn as text, never as markup', async () => {
  await openLive();
  expect(document.querySelector('[data-timeline] img')).toBeNull();
  expect(document.querySelector('[data-timeline] script')).toBeNull();
  expect(document.body.textContent).toContain('<img src=x onerror="window.reached = true">');
  expect(document.querySelector('.subject')?.textContent).toBe('<script>window.reached = true</script>');
  expect((window as { reached?: boolean }).reached).toBeUndefined();
});

it('THR-030: an answer to a request goes to the runner of this computer with the ID of the runner', async () => {
  const shell = await openLive();
  const allow = [...document.querySelectorAll('button')].find((button) => button.textContent.includes('Allow once'));
  allow?.click();
  await settle();
  expect(shell.calls.at(-1)).toEqual({ call: 'answer', thread: THREAD, request: '0190a000-0000-7000-8000-0000000000aa', answer: { answer: 'allow' } });
});

it('THR-017: Enter in the composer sends the prompt; Shift+Enter does not', async () => {
  const shell = await openLive();
  const field = document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Prompt"]');
  if (field === null) throw new Error('no composer');
  field.value = 'Go on';
  field.dispatchEvent(new Event('input', { bubbles: true }));
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, bubbles: true }));
  await settle();
  expect(shell.calls.filter((call) => call.call === 'send')).toEqual([]);
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  await settle();
  expect(shell.calls.filter((call) => call.call === 'send')).toEqual([{ call: 'send', thread: THREAD, text: 'Go on' }]);
});
