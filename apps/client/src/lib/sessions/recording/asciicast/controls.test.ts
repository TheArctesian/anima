// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import Controls from './Controls.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function show(props: { playing: boolean; position: number }) {
  const calls = { play: vi.fn(), pause: vi.fn(), seek: vi.fn() };
  app = mount(Controls, { target: document.body, props: { ...props, duration: 125, onplay: calls.play, onpause: calls.pause, onseek: calls.seek } });
  flushSync();
  return calls;
}

it('D-12: the button is named Play while paused and calls play', () => {
  const calls = show({ playing: false, position: 0 });
  const button = document.querySelector('button');
  expect(button?.getAttribute('aria-label')).toBe('Play');
  button?.click();
  expect(calls.play).toHaveBeenCalledOnce();
});

it('D-12: the button is named Pause while playing and calls pause', () => {
  const calls = show({ playing: true, position: 3 });
  const button = document.querySelector('button');
  expect(button?.getAttribute('aria-label')).toBe('Pause');
  button?.click();
  expect(calls.pause).toHaveBeenCalledOnce();
});

it('D-12: the slider has a name, a value text and the elapsed and total time as text, and a change seeks', () => {
  const calls = show({ playing: false, position: 65 });
  const slider = document.querySelector('input');
  expect(slider?.type).toBe('range');
  expect(slider?.getAttribute('aria-label')).toBe('Position in the recording');
  expect(slider?.getAttribute('aria-valuetext')).toBe('1:05 of 2:05');
  expect(document.querySelector('.time')?.textContent).toBe('1:05 / 2:05');
  if (slider === null) throw new Error('no slider');
  slider.value = '30';
  slider.dispatchEvent(new Event('input', { bubbles: true }));
  expect(calls.seek).toHaveBeenCalledWith(30);
});
