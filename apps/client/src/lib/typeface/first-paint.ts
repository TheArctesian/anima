import type { TextRole, Typeface, TypefaceChoice } from './typeface';

/** The faces the first paint uses for each typeface of each role, as CSS font shorthands `FontFaceSet.load` takes. */
const FACES: { readonly [R in TextRole]: Readonly<Record<Typeface<R>, readonly string[]>> } = {
  ui: {
    'ibm-plex-sans': ['200 1em "IBM Plex Sans"', '400 1em "IBM Plex Sans"', '500 1em "IBM Plex Sans"'],
    opendyslexic: ['400 1em "OpenDyslexic Nerd Font Propo"'],
  },
  mono: {
    'ibm-plex-mono': ['400 1em "IBM Plex Mono"'],
    opendyslexic: ['400 1em "OpenDyslexicM Nerd Font Mono"'],
  },
};

/** Every face any choice can use at first paint, for the check that each one is declared in `fonts.css`. */
export const ALL_FIRST_PAINT_FACES: readonly string[] = [
  ...new Set([...Object.values(FACES.ui).flat(), ...Object.values(FACES.mono).flat()]),
];

/** The faces the first paint of `choice` uses: a font that arrives after the first render moves the text (MOT-007). */
export function firstPaintFaces(choice: TypefaceChoice): readonly string[] {
  return [...new Set([...FACES.ui[choice.ui], ...FACES.mono[choice.mono]])];
}

/** The most a first render waits for its fonts: about the block period browsers use with `font-display: block` (R-042). */
export const FONT_WAIT_MS = 3000;

/**
 * Resolves when the first-paint faces of `choice` have loaded or failed, or after `FONT_WAIT_MS`. An engine without
 * font loading (`fonts` undefined) does not wait; a font that cannot load leaves the fallback family in use.
 */
export async function waitForFirstPaintFonts(choice: TypefaceChoice, fonts: Pick<FontFaceSet, 'load'> | undefined): Promise<void> {
  if (fonts === undefined) return;
  const loaded = Promise.allSettled(firstPaintFaces(choice).map((face) => fonts.load(face)));
  let timer: ReturnType<typeof setTimeout> | undefined;
  const expired = new Promise<void>((resolve) => {
    timer = setTimeout(resolve, FONT_WAIT_MS);
  });
  try {
    await Promise.race([loaded, expired]);
  } finally {
    clearTimeout(timer);
  }
}
