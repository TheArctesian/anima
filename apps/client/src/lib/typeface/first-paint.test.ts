import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ALL_FIRST_PAINT_FACES, FONT_WAIT_MS, firstPaintFaces, waitForFirstPaintFonts } from './first-paint';
import type { TypefaceChoice } from './typeface';

const DEFAULTS: TypefaceChoice = { ui: 'ibm-plex-sans', mono: 'ibm-plex-mono' };

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

/** Whether the promise has settled, without waiting for it. */
async function settled(promise: Promise<void>): Promise<boolean> {
  let done = false;
  void promise.then(() => {
    done = true;
  });
  await vi.advanceTimersByTimeAsync(0);
  return done;
}

it('UI-020: the first render waits for every face the chosen typefaces use', async () => {
  const load = vi.fn<(face: string) => Promise<FontFace[]>>().mockResolvedValue([]);
  await waitForFirstPaintFonts({ ui: 'opendyslexic', mono: 'ibm-plex-mono' }, { load });
  expect(load.mock.calls.map(([face]) => face).toSorted()).toEqual(
    firstPaintFaces({ ui: 'opendyslexic', mono: 'ibm-plex-mono' }).toSorted(),
  );
  expect(firstPaintFaces(DEFAULTS)).toEqual(['200 1em "IBM Plex Sans"', '400 1em "IBM Plex Sans"', '500 1em "IBM Plex Sans"', '400 1em "IBM Plex Mono"']);
});

it('MOT-007: it resolves once the fonts have loaded, without the cap', async () => {
  const load = vi.fn().mockResolvedValue([]);
  expect(await settled(waitForFirstPaintFonts(DEFAULTS, { load }))).toBe(true);
  expect(vi.getTimerCount()).toBe(0);
});

it('MOT-007: a font that fails to load does not stop the render', async () => {
  const load = vi.fn().mockRejectedValue(new Error('network'));
  expect(await settled(waitForFirstPaintFonts(DEFAULTS, { load }))).toBe(true);
});

it('MOT-007: a font that never settles holds the render for FONT_WAIT_MS and no longer', async () => {
  const load = vi.fn().mockReturnValue(new Promise(() => undefined));
  const waiting = waitForFirstPaintFonts(DEFAULTS, { load });
  await vi.advanceTimersByTimeAsync(FONT_WAIT_MS - 1);
  expect(await settled(waiting)).toBe(false);
  await vi.advanceTimersByTimeAsync(1);
  expect(await settled(waiting)).toBe(true);
});

it('MOT-007: an engine without font loading renders at once', async () => {
  expect(await settled(waitForFirstPaintFonts(DEFAULTS, undefined))).toBe(true);
  expect(vi.getTimerCount()).toBe(0);
});

it('UI-010: every face the wait names is declared by a bundled @font-face, so a renamed family cannot disable the wait', () => {
  const css = readFileSync('src/styles/fonts.css', 'utf8');
  const declared = [...css.matchAll(/@font-face\s*{([^}]*)}/g)].map((block) => ({
    family: /font-family:\s*"([^"]+)"/.exec(block[1] ?? '')?.[1],
    weight: /font-weight:\s*(\d+)/.exec(block[1] ?? '')?.[1],
  }));
  expect(ALL_FIRST_PAINT_FACES.length).toBeGreaterThan(0);
  for (const face of ALL_FIRST_PAINT_FACES) {
    const [weight, family] = /^(\d+) \S+ "([^"]+)"$/.exec(face)?.slice(1) ?? [];
    expect(declared, face).toContainEqual({ family, weight });
  }
});
