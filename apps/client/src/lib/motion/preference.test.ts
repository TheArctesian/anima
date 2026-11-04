import { afterEach, expect, it, vi } from 'vitest';
import { motionPreference } from './preference';

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

afterEach(() => {
  vi.unstubAllGlobals();
});

it('MOT-040: without a window there is no reduced-motion preference', () => {
  expect(motionPreference().reduced).toBe(false);
});

it('MOT-041: the preference reads prefers-reduced-motion and sees a change', () => {
  const media = new FakeMedia('', true);
  const matchMedia = vi.fn<(query: string) => FakeMedia>(() => media);
  vi.stubGlobal('matchMedia', matchMedia);

  const preference = motionPreference();
  expect(preference.reduced).toBe(true);
  media.change(false);
  expect(preference.reduced).toBe(false);
  expect(motionPreference()).toBe(preference);
  expect(matchMedia).toHaveBeenCalledOnce();
  expect(matchMedia).toHaveBeenCalledWith('(prefers-reduced-motion: reduce)');
});
