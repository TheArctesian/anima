import { storedTypefaces, type Typeface } from '$lib/typeface/typeface';

export const prerender = false;
export const ssr = false;

/** The faces the first paint uses, by chosen typeface: a font that arrives after the first render moves the text (MOT-007). */
const FACES: Readonly<Record<Typeface, { ui: readonly string[]; mono: readonly string[] }>> = {
  'fira-code': {
    ui: ['400 1em "Fira Code"', '700 1em "Fira Code"'],
    mono: ['400 1em "Fira Code"'],
  },
  opendyslexic: {
    ui: ['400 1em "OpenDyslexic Nerd Font Propo"', '700 1em "OpenDyslexic Nerd Font Propo"'],
    mono: ['400 1em "OpenDyslexicM Nerd Font Mono"'],
  },
};

/** Icons are Nerd Font glyphs drawn from this face (UI-014, `font-icon`); an icon's width changes when it arrives. */
const ICON_FACE = '400 1em "OpenDyslexic Nerd Font Propo"';

/** The most a first render waits for its fonts: the length of the `font-display: block` period (R-042). */
const FONT_WAIT_MS = 3000;

/**
 * The first render waits for the chosen fonts, so the text is laid out in its final font and no later swap moves the
 * page (MOT-007: CLS 0). The files are bundled, so the wait is short; a failed or slow file ends it at once or after
 * `FONT_WAIT_MS`, and the page then renders in the fallback font.
 */
export async function load(): Promise<void> {
  const choice = storedTypefaces(localStorage);
  const faces = new Set([ICON_FACE, ...FACES[choice.ui].ui, ...FACES[choice.mono].mono]);
  const loaded = Promise.allSettled([...faces].map((face) => document.fonts.load(face)));
  const expired = new Promise<void>((resolve) => setTimeout(resolve, FONT_WAIT_MS));
  await Promise.race([loaded, expired]);
}
