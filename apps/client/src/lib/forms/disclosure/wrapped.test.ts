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
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
});

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

/** `<form><div wrapper><Disclosure/></div><div below/></form>`: the section is the last child of its wrapper. */
function wrapped(level?: 3) {
  const form = document.createElement('form');
  const wrapper = document.createElement('div');
  const below = document.createElement('div');
  form.append(wrapper, below);
  document.body.append(form);
  app = mount(Disclosure, { target: wrapper, props: { label: 'More', children: content, preference: { reduced: false }, level } });
  flushSync();
  const region = wrapper.querySelector('.region') as HTMLElement;
  below.getBoundingClientRect = () => new DOMRect(0, region.hidden ? 40 : 100, 100, 20);
  return { wrapper, below, button: wrapper.querySelector('button') as HTMLButtonElement };
}

async function settle(): Promise<void> {
  for (let pass = 0; pass < 3; pass += 1) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

it('5.7.3 disclosure: inside a wrapper the elements after the wrapper move by FLIP', async () => {
  const { button } = wrapped();

  button.click();
  await settle();

  const springs = runs.filter((run) => run.options.type === 'spring').map((run) => run.keyframes);
  expect(springs).toEqual([
    [0, 0],
    [-60, 0],
  ]);
});

/**
 * The device Settings tab: `<div><form><div wrapper><Disclosure/></div><div actions/></form><p note/><section delete/></div>`.
 * The section's height moves the form's actions and everything after the form; `still` sits beside and does not move.
 */
function settingsTab() {
  const page = document.createElement('div');
  const form = document.createElement('form');
  const wrapper = document.createElement('div');
  const actions = document.createElement('div');
  const remove = document.createElement('section');
  const still = document.createElement('aside');
  form.append(wrapper, actions);
  page.append(form, remove);
  document.body.append(page, still);
  app = mount(Disclosure, { target: wrapper, props: { label: 'SSH', children: content, preference: { reduced: false } } });
  flushSync();
  const region = wrapper.querySelector('.region') as HTMLElement;
  const placed = (closedTop: number) => () => new DOMRect(0, region.hidden ? closedTop : closedTop + 500, 100, 20);
  actions.getBoundingClientRect = placed(100);
  remove.getBoundingClientRect = placed(200);
  still.getBoundingClientRect = () => new DOMRect(900, 0, 100, 20);
  return { button: wrapper.querySelector('button') as HTMLButtonElement, actions, remove, still };
}

it('MOT-007: the elements after the form move by FLIP too, so the Delete device button does not jump', async () => {
  const { button, actions, remove } = settingsTab();

  button.click();
  await settle();

  const moved = runs.filter((run) => run.options.type === 'spring').map((run) => run.keyframes);
  expect(moved).toEqual([
    [0, 0],
    [-500, 0],
    [0, 0],
    [-500, 0],
  ]);
  expect(actions.style.translate).not.toBe('');
  expect(remove.style.translate).not.toBe('');
});

it('5.7.3 disclosure: an element whose place does not change gets no translate', async () => {
  const { button, still } = settingsTab();

  button.click();
  await settle();

  expect(still.style.translate).toBe('');
});

it('5.7.3 disclosure: with a heading level the button sits in that heading', () => {
  const { wrapper } = wrapped(3);
  expect(wrapper.querySelector('h3 > button')?.textContent).toContain('More');
});

it('5.7.3 disclosure: an old fade that ends after a reopen leaves the content visible', async () => {
  const { wrapper, button } = wrapped();
  const region = wrapper.querySelector('.region') as HTMLElement;
  button.click();
  await settle();
  button.click();
  await settle();
  const closing = runs.at(-1);

  button.click();
  await settle();
  closing?.finish();
  await settle();

  expect(button.getAttribute('aria-expanded')).toBe('true');
  expect(region.hidden).toBe(false);
  expect(region.inert).toBe(false);
});
