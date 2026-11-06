// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import Disclosure from './Disclosure.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const content = createRawSnippet(() => ({ render: () => '<p>settings</p>' }));
/** The section's height when open. */
const SECTION = 32;
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
});

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function translateY(element: HTMLElement): number {
  return Number.parseFloat(element.style.translate.split(' ')[1] ?? '0') || 0;
}

/** When the browser moves the scroll position back: at the close, or once the fading content is hidden. */
type Clamp = 'none' | 'at close' | 'when hidden';

/**
 * A modal's scroll body at its scroll end: `<div body><div form><div wrapper><Disclosure/></div><div next/></div></div>`.
 * Closing the section moves `next` up by its height; a clamp moves the whole form down by the same amount,
 * so on screen `next` ends where it started. Each box on screen is its layout box moved by its own
 * `translate` and by the form's.
 */
function scrolledToEnd(clamp: Clamp) {
  const body = document.createElement('div');
  body.style.overflowY = 'auto';
  const form = document.createElement('div');
  const wrapper = document.createElement('div');
  const next = document.createElement('div');
  form.append(wrapper, next);
  body.append(form);
  document.body.append(body);
  app = mount(Disclosure, { target: wrapper, props: { label: 'SSH', children: content, preference: { reduced: false }, open: true } });
  flushSync();
  const region = wrapper.querySelector('.region') as HTMLElement;
  const gone = () => region.hidden || region.classList.contains('leaving');
  const scrolledBack = () => (clamp === 'at close' ? gone() : clamp === 'when hidden' && region.hidden) ? SECTION : 0;
  form.getBoundingClientRect = () => new DOMRect(0, scrolledBack() + translateY(form), 100, 300);
  next.getBoundingClientRect = () => new DOMRect(0, 200 - (gone() ? SECTION : 0) + scrolledBack() + translateY(form) + translateY(next), 100, 20);
  return { form, next, region, button: wrapper.querySelector('button') as HTMLButtonElement };
}

async function settle(): Promise<void> {
  for (let pass = 0; pass < 3; pass += 1) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

/** Ends the content's fade, as the engine would after `duration-base`. */
async function endFade(): Promise<void> {
  for (const run of runs.filter((candidate) => candidate.options.type !== 'spring')) run.finish();
  await settle();
}

const springs = () => runs.filter((run) => run.options.type === 'spring').map((run) => run.keyframes);

it('MOT-007: when a close moves the scroll position back, the content springs from where it was and the row below does not step', async () => {
  const { form, next, button } = scrolledToEnd('at close');

  button.click();
  await settle();

  // The form starts where it was (the clamp undone by its translate) and springs to its new place.
  expect(form.getBoundingClientRect().top).toBe(0);
  expect(springs()).toContainEqual([-SECTION, 0]);
  // The row below did not move on screen: it gets a translate of its own against the form's, so it is not carried.
  expect(next.getBoundingClientRect().top).toBe(200);
  expect(springs()).toContainEqual([SECTION, 0]);
});

it('MOT-007: when the scroll position moves back as the faded content is hidden, the content springs from where it was', async () => {
  const { form, button } = scrolledToEnd('when hidden');
  button.click();
  await settle();
  expect(springs()).not.toContainEqual([-SECTION, 0]);

  await endFade();

  expect(form.getBoundingClientRect().top).toBe(0);
  expect(springs()).toContainEqual([-SECTION, 0]);
});

it('5.7.3 disclosure: with no clamp and nothing after the section, nothing gets a translate and no spring runs', async () => {
  const { form, next, button } = scrolledToEnd('none');
  next.remove();

  button.click();
  await settle();
  await endFade();

  expect(springs()).toEqual([]);
  expect(form.style.translate).toBe('');
});

it('5.7.3 disclosure: reopened during the fade in a scroll body, the content is visible and the form is in place', async () => {
  const { form, region, button } = scrolledToEnd('when hidden');
  button.click();
  await settle();
  button.click();
  await settle();

  await endFade();

  expect(button.getAttribute('aria-expanded')).toBe('true');
  expect(region.hidden).toBe(false);
  expect(region.inert).toBe(false);
  expect(form.getBoundingClientRect().top).toBe(0);
});
