// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '$lib/motion/engine.fixture';
import { ease } from '$lib/motion/tokens';
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

/** The runs that fade the notice: a tween over opacity, not a spring over a place. */
const fades = () => runs.filter((run) => run.options['duration'] !== undefined);

it('5.7.3 Disclosure: the notice starts invisible and fades in over duration-base with ease-enter', async () => {
  const { props, notice } = screen(null);
  props.message = 'The connection closed';
  await tick();
  await tick();
  expect(fades().map((run) => [run.keyframes, run.options])).toEqual([[[0, 1], { duration: 0.2, ease: ease.enter }]]);
  expect(notice()?.style.opacity).toBe('0');
  fades()[0]?.finish();
  expect(notice()?.style.opacity).toBe('1');
});

it('MOT-040: under reduced motion the list is in its new place at once, with no spring, and the notice still fades in', async () => {
  const { props, list, notice } = screen(null, true);
  props.message = 'The connection closed';
  await tick();
  await tick();
  expect(list.getBoundingClientRect().top).toBe(HEIGHT);
  expect(runs.filter((run) => run.options['stiffness'] !== undefined)).toEqual([]);
  expect(fades().map((run) => [run.keyframes, run.options])).toEqual([[[0, 1], { duration: 0.2, ease: ease.enter }]]);
  expect(notice()).not.toBeNull();
});

it('NET-004: a new message while the notice shows replaces the text and moves nothing', async () => {
  const { props, list, notice } = screen('The connection closed');
  props.message = 'The hub answered with an error';
  await tick();
  await tick();
  expect(document.querySelectorAll('.notice')).toHaveLength(1);
  expect(notice()?.textContent).toBe('Live updates stopped: The hub answered with an error');
  expect(runs).toEqual([]);
  expect(list.style.translate).toBe('');
});

it('NET-004: a message that is there at mount shows at once, with no motion', () => {
  const { list, notice } = screen('The connection closed');
  expect(notice()?.textContent).toBe('Live updates stopped: The connection closed');
  expect(runs).toEqual([]);
  expect(list.style.translate).toBe('');
});

it('NET-004: the subject names what stopped, so the log panel says logs and a device screen says updates', async () => {
  const target = document.body.appendChild(document.createElement('div'));
  app = mount(LiveNotice, { target, props: { message: 'The connection closed', subject: 'logs', gap: 'padded' } });
  flushSync();
  expect(target.querySelector('.notice')?.textContent).toBe('Live logs stopped: The connection closed');
});
