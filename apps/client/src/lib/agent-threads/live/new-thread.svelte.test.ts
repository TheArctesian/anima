// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import NewThread from '../view/NewThread.svelte';
import type { CallResult } from './live.svelte';
import type { Action } from './run';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
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

function type(field: HTMLInputElement | HTMLTextAreaElement, text: string): void {
  field.value = text;
  field.dispatchEvent(new Event('input', { bubbles: true }));
}

it('THR-046: a folder whose files run commands by themselves shows its notice before the thread starts', async () => {
  const calls: Action[] = [];
  const onStart = (action: Action): Promise<CallResult> => {
    calls.push(action);
    if (action.call === 'folder') return Promise.resolve({ ok: true, outcome: { outcome: 'folder', notice: ['The project files of this folder make Claude Code do these things with no question:', 'Runs the command `curl x | sh` at SessionStart.'] } });
    return Promise.resolve({ ok: true, outcome: { outcome: 'started', thread: 't-1' } });
  };
  const started: string[] = [];
  app = mount(NewThread, { target: document.body, props: { open: true, onStart, onStarted: (thread: string) => started.push(thread), onClose: () => undefined } });
  flushSync();
  const folder = document.querySelector<HTMLInputElement>('input[placeholder="/Users/you/project"]');
  const prompt = document.querySelector<HTMLTextAreaElement>('textarea[aria-label="First prompt"]');
  if (folder === null || prompt === null) throw new Error('the dialog has its fields');
  type(folder, '/tmp/project');
  type(prompt, 'Hello');
  flushSync();
  const form = document.querySelector('form');
  form?.requestSubmit();
  await settle();
  expect(calls.map((call) => call.call)).toEqual(['folder']);
  expect(document.querySelector('[role="note"]')?.textContent).toContain('curl x | sh');
  form?.requestSubmit();
  await settle();
  expect(calls.map((call) => call.call)).toEqual(['folder', 'start']);
  expect(started).toEqual(['t-1']);
});

it('THR-046: a folder check that fails is said and does not stop the start; the same line twice is shown twice', async () => {
  const calls: Action[] = [];
  let failing = true;
  const onStart = (action: Action): Promise<CallResult> => {
    calls.push(action);
    if (action.call === 'folder' && failing) return Promise.resolve({ ok: false, words: 'The runner did not answer.' });
    if (action.call === 'folder') return Promise.resolve({ ok: true, outcome: { outcome: 'folder', notice: ['Head:', 'Runs the command `x` at SessionStart.', 'Runs the command `x` at SessionStart.'] } });
    return Promise.resolve({ ok: true, outcome: { outcome: 'started', thread: 't-2' } });
  };
  app = mount(NewThread, { target: document.body, props: { open: true, onStart, onStarted: () => undefined, onClose: () => undefined } });
  flushSync();
  const folder = document.querySelector<HTMLInputElement>('input[placeholder="/Users/you/project"]');
  const prompt = document.querySelector<HTMLTextAreaElement>('textarea[aria-label="First prompt"]');
  if (folder === null || prompt === null) throw new Error('the dialog has its fields');
  type(folder, '/tmp/a');
  type(prompt, 'Hello');
  flushSync();
  const form = document.querySelector('form');
  form?.requestSubmit();
  await settle();
  expect(document.querySelector('[role="note"]')?.textContent).toContain('could not be checked');
  form?.requestSubmit();
  await settle();
  expect(calls.map((call) => call.call)).toEqual(['folder', 'start']);
  failing = false;
  type(folder, '/tmp/b');
  type(prompt, 'Again');
  flushSync();
  form?.requestSubmit();
  await settle();
  expect(document.querySelectorAll('[role="note"] li')).toHaveLength(2);
});
