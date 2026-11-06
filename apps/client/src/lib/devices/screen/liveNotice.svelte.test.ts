// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '$lib/motion/engine.fixture';
import LiveNotice from './LiveNotice.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const HEIGHT = 36;
let app: ReturnType<typeof mount> | undefined;

/** A notice slot with a list after it whose top is pushed down while the notice is in the layout. */
function screen(initial: string | null, reduced = false) {
  const target = document.body.appendChild(document.createElement('div'));
  const props = $state({ message: initial, preference: { reduced } });
  app = mount(LiveNotice, { target, props });
  flushSync();
  const list = target.appendChild(document.createElement('div'));
  list.getBoundingClientRect = () => {
    const notice = target.querySelector('.notice');
    const inFlow = notice !== null && !notice.classList.contains('leaving');
    const [x = 0, y = 0] = list.style.translate.split(/\s+/).filter((part) => part !== '').map((part) => Number.parseFloat(part));
    return new DOMRect(x, (inFlow ? HEIGHT : 0) + y, 100, 40);
  };
  return { props, list, notice: () => target.querySelector<HTMLElement>('.notice') };
}

beforeEach(() => {
  runs.length = 0;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('NET-004: no notice while live updates run, and its role is status when they stop', async () => {
  const { props, notice } = screen(null);
  expect(notice()).toBeNull();
  props.message = 'The connection closed';
  await tick();
  expect(notice()?.getAttribute('role')).toBe('status');
  expect(notice()?.textContent).toBe('Live updates stopped: The connection closed');
});

it('MOT-007: when the notice appears the list starts where it was and moves by spring, never a one-frame step', async () => {
  const { props, list } = screen(null);
  props.message = 'The connection closed';
  await tick();
  await tick();
  // The list is HEIGHT lower in the layout; it is held HEIGHT above that and springs to it.
  expect(list.style.translate).toBe(`0px ${String(-HEIGHT)}px`);
  expect(runs.some((run) => run.keyframes.at(-1) === 0)).toBe(true);
});

it('MOT-007: when the notice goes it fades where it was while the list moves up by spring', async () => {
  const { props, list, notice } = screen('The connection closed');
  props.message = null;
  await tick();
  await tick();
  expect(notice()?.classList.contains('leaving')).toBe(true);
  expect(list.style.translate).toBe(`0px ${String(HEIGHT)}px`);
  for (const run of runs) run.finish();
  await new Promise((done) => setTimeout(done));
  expect(notice()).toBeNull();
});

it('MOT-002: a double toggle leaves one notice, shown again from where its fade had got to', async () => {
  const { props, notice } = screen('The connection closed');
  props.message = null;
  await tick();
  await tick();
  props.message = 'Still closed';
  await tick();
  await tick();
  for (const run of runs) run.finish();
  await new Promise((done) => setTimeout(done));
  expect(document.querySelectorAll('.notice')).toHaveLength(1);
  expect(notice()?.classList.contains('leaving')).toBe(false);
  expect(notice()?.textContent).toBe('Live updates stopped: Still closed');
});

it('MOT-040: under reduced motion the list is in its new place at once and the notice still fades', async () => {
  const { props, list } = screen(null, true);
  props.message = 'The connection closed';
  await tick();
  await tick();
  expect(list.getBoundingClientRect().top).toBe(HEIGHT);
  expect(runs.every((run) => run.keyframes[0] === run.keyframes.at(-1) || run.options['duration'] !== undefined)).toBe(true);
});
