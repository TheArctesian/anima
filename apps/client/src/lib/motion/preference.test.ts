// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';

class FakeMedia extends EventTarget {
  constructor(
    readonly media: string,
    public matches: boolean,
  ) {
    super();
  }

  change(matches: boolean): void {
    this.matches = matches;
    this.dispatchEvent(Object.assign(new Event('change'), { matches }));
  }
}

/** A fresh module, since the preference is shared once made. */
async function motionPreference() {
  return (await import('./preference')).motionPreference();
}

/** The OS setting as `prefers-reduced-motion` reports it. */
function system(reduces: boolean): FakeMedia {
  const media = new FakeMedia('', reduces);
  vi.stubGlobal(
    'matchMedia',
    vi.fn<(query: string) => FakeMedia>(() => media),
  );
  return media;
}

beforeEach(() => {
  vi.resetModules();
});

afterEach(() => {
  vi.unstubAllGlobals();
  delete document.documentElement.dataset['motion'];
});

it('MOT-040: without a document there is no reduced-motion preference', async () => {
  vi.stubGlobal('document', undefined);
  expect((await motionPreference()).reduced).toBe(false);
});

it('MOT-040: without prefers-reduced-motion and with no choice, motion is full', async () => {
  expect((await motionPreference()).reduced).toBe(false);
});

it('MOT-041: the preference reads prefers-reduced-motion and sees a change', async () => {
  const media = system(true);
  const preference = await motionPreference();
  expect(preference.reduced).toBe(true);
  media.change(false);
  expect(preference.reduced).toBe(false);
  expect((await import('./preference')).motionPreference()).toBe(preference);
  expect(matchMedia).toHaveBeenCalledOnce();
  expect(matchMedia).toHaveBeenCalledWith('(prefers-reduced-motion: reduce)');
});

it('MOT-043: System (no data-motion) leaves the media query in charge', async () => {
  const media = system(false);
  const preference = await motionPreference();
  expect(preference.reduced).toBe(false);
  media.change(true);
  expect(preference.reduced).toBe(true);
});

it('MOT-043: On (data-motion="reduce") reduces the springs while the OS has no preference', async () => {
  system(false);
  document.documentElement.dataset['motion'] = 'reduce';
  expect((await motionPreference()).reduced).toBe(true);
});

it('MOT-043: Off (data-motion="full") keeps full motion while the OS asks to reduce', async () => {
  const media = system(true);
  document.documentElement.dataset['motion'] = 'full';
  const preference = await motionPreference();
  expect(preference.reduced).toBe(false);
  media.change(true);
  expect(preference.reduced).toBe(false);
});

it('MOT-043: a choice made after the preference exists applies at once, and taking it away hands back to the OS', async () => {
  system(true);
  const preference = await motionPreference();
  document.documentElement.dataset['motion'] = 'full';
  expect(preference.reduced).toBe(false);
  delete document.documentElement.dataset['motion'];
  expect(preference.reduced).toBe(true);
});

it('MOT-043: an unknown data-motion value is System', async () => {
  system(true);
  document.documentElement.dataset['motion'] = 'sometimes';
  expect((await motionPreference()).reduced).toBe(true);
});
