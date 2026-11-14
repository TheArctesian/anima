// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { choiceOf, type Choice } from '$lib/forms/segmented/choice.fixture';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { springMotion } from '$lib/motion/options';
import { motionPreference } from '$lib/motion/preference';
import MotionRow from './MotionRow.svelte';

let app: ReturnType<typeof mount> | undefined;

function show(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(MotionRow, { target });
  flushSync();
  return target;
}

function motionSelect(target: HTMLElement): Choice {
  return choiceOf(target, 'Reduce motion');
}

function choose(control: Choice, value: string): void {
  control.pick(value);
}

async function restart(): Promise<HTMLElement> {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  return show();
}

/** The OS setting as `prefers-reduced-motion` reports it; the shared preference keeps the first query it gets. */
const os = Object.assign(new EventTarget(), { media: '(prefers-reduced-motion: reduce)', matches: false });

/** The attribute observer reports on a microtask, as a browser's does. */
async function observed(): Promise<void> {
  await Promise.resolve();
  flushSync();
}

beforeEach(() => {
  os.matches = false;
  vi.stubGlobal('matchMedia', () => os);
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  delete document.documentElement.dataset['motion'];
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('MOT-043: the labelled Reduce motion choice offers System, On and Off, and starts at System', () => {
  const control = motionSelect(show());
  expect(control.options.map((option) => option.textContent)).toEqual(['System', 'On', 'Off']);
  expect(control.value).toBe('system');
  expect(document.documentElement.hasAttribute('data-motion')).toBe(false);
});

it('MOT-043: each choice sets data-motion on <html> at once and is still chosen after a restart', async () => {
  for (const [choice, attribute] of [['reduce', 'reduce'], ['full', 'full'], ['system', null]] as const) {
    choose(motionSelect(await restart()), choice);
    expect(document.documentElement.getAttribute('data-motion')).toBe(attribute);
    expect(localStorage.getItem('anima.motion')).toBe(choice);
    expect(motionSelect(await restart()).value).toBe(choice);
  }
});

it('MOT-043: an unknown stored value shows System', () => {
  localStorage.setItem('anima.motion', 'sometimes');
  expect(motionSelect(show()).value).toBe('system');
});

it('MOT-043: a storage that refuses to save still applies the choice and says it will not last', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(motionSelect(target), 'reduce');
  expect(document.documentElement.dataset['motion']).toBe('reduce');
  expect(motionSelect(target).value).toBe('reduce');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(
    'Anima could not save this choice on this device; it lasts until the app restarts.',
  );
});

it('MOT-043: the "could not save" message goes once a later choice is saved', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementationOnce(() => {
    throw new Error('quota');
  });
  const target = show();
  choose(motionSelect(target), 'reduce');
  expect(target.querySelector('[role="alert"]')).not.toBeNull();
  choose(motionSelect(target), 'full');
  expect(target.querySelector('[role="alert"]')).toBeNull();
  expect(localStorage.getItem('anima.motion')).toBe('full');
});

it('MOT-041, MOT-043: On makes the JS springs instant while the OS has no preference, with no reload', async () => {
  const preference = motionPreference();
  const seen: boolean[] = [];
  const stop = $effect.root(() => {
    $effect(() => {
      seen.push(preference.reduced);
    });
  });
  flushSync();
  const control = motionSelect(show());

  choose(control, 'reduce');
  await observed();
  expect(springMotion('default', preference.reduced).type).toBe('instant');

  choose(control, 'system');
  await observed();
  stop();
  expect(springMotion('default', preference.reduced).type).toBe('spring');
  expect(seen).toEqual([false, true, false]);
});

it('MOT-041, MOT-043: Off keeps the JS springs while the OS asks to reduce', () => {
  os.matches = true;
  const preference = motionPreference();
  expect(preference.reduced).toBe(true);
  choose(motionSelect(show()), 'full');
  expect(springMotion('default', preference.reduced).type).toBe('spring');
});
