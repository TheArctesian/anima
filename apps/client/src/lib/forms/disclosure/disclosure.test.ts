// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { MotionPreference } from '$lib/motion/preference';
import { runs } from '../../motion/engine.fixture';
import Disclosure from './Disclosure.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const content = createRawSnippet(() => ({ render: () => '<p>settings</p>' }));
const CLOSED_TOP = 40;
const OPEN_TOP = 100;
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
});

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

/** The section, plus an element after it whose place depends on whether the content is in the layout. */
function show(preference: MotionPreference = { reduced: false }) {
  app = mount(Disclosure, { target: document.body, props: { label: 'Advanced', children: content, preference } });
  flushSync();
  const below = document.createElement('div');
  document.body.append(below);
  const region = document.querySelector('.region') as HTMLElement;
  below.getBoundingClientRect = () => {
    const inLayout = !region.hidden && !region.classList.contains('leaving');
    return new DOMRect(0, inLayout ? OPEN_TOP : CLOSED_TOP, 100, 20);
  };
  return { button: document.querySelector('button') as HTMLButtonElement, region, below };
}

async function settle(): Promise<void> {
  for (let pass = 0; pass < 3; pass += 1) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

async function click(target: HTMLElement): Promise<void> {
  target.click();
  await settle();
}

const springs = () => runs.filter((run) => run.options.type === 'spring');
const fades = () => runs.filter((run) => 'duration' in run.options);

it('5.7.3 disclosure: the button controls a content region that starts closed', () => {
  const { button, region } = show();
  expect(button.getAttribute('aria-expanded')).toBe('false');
  expect(button.getAttribute('aria-controls')).toBe(region.id);
  expect(region.hidden).toBe(true);
});

it('5.7.3 disclosure: on open the content fades in and what is below moves by FLIP on a spring', async () => {
  const { button, region } = show();

  await click(button);

  expect(button.getAttribute('aria-expanded')).toBe('true');
  expect(region.hidden).toBe(false);
  expect(springs().map((run) => run.keyframes)).toEqual([
    [0, 0],
    [CLOSED_TOP - OPEN_TOP, 0],
  ]);
  expect(fades().map((run) => run.keyframes)).toEqual([[0, 1]]);
});

it('5.7.3 disclosure: on close the content leaves the layout at once, so what is below moves with no snap at the end', async () => {
  const { button, region, below } = show();
  await click(button);
  fades().at(-1)?.finish();
  await settle();
  runs.length = 0;

  await click(button);

  expect(button.getAttribute('aria-expanded')).toBe('false');
  expect(region.classList.contains('leaving')).toBe(true);
  expect(region.inert).toBe(true);
  expect(region.hidden).toBe(false);
  expect(springs().map((run) => run.keyframes)).toEqual([
    [0, 0],
    [OPEN_TOP - CLOSED_TOP, 0],
  ]);
  expect(below.getBoundingClientRect().top).toBe(CLOSED_TOP);
  expect(fades().map((run) => run.keyframes)).toEqual([[1, 0]]);
});

it('5.7.3 disclosure: the content is taken out of the document when its fade ends', async () => {
  const { button, region } = show();
  await click(button);
  await click(button);

  fades().at(-1)?.finish();
  await settle();

  expect(region.hidden).toBe(true);
  expect(region.classList.contains('leaving')).toBe(false);
});

it('5.7.3 disclosure: reopening while it fades out keeps the content', async () => {
  const { button, region } = show();
  await click(button);
  await click(button);
  const closing = fades().at(-1);

  await click(button);
  closing?.finish();
  await settle();

  expect(button.getAttribute('aria-expanded')).toBe('true');
  expect(region.hidden).toBe(false);
});

it('MOT-040: under reduced motion the layout change is instant and only the fade takes duration-base', async () => {
  const { button, region } = show({ reduced: true });

  await click(button);

  expect(region.hidden).toBe(false);
  expect(springs()).toEqual([]);
  expect(fades().map((run) => run.options.duration)).toEqual([0.2]);
});

it('MOT-006: nothing but translate and opacity is animated, and no height', async () => {
  const { button, region } = show();
  await click(button);
  await click(button);

  expect(region.style.height).toBe('');
  expect(runs.every((run) => run.keyframes.length === 2)).toBe(true);
});
